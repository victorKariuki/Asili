//! REPL: read-eval-print loop with persistent env. Use `?topic` to show docs from docs/repl/{topic}.md.

use super::{CliError, CliResult};
use asili_evaluator::{run_block_in_env_with_telemetry, Env, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, Stmt};
use std::io::{self, BufRead, Write};
use std::path::Path;

const REPL_FUNC: &str = "__repl__";
const PROMPT: &str = "> ";

pub fn run(_args: &[String]) -> CliResult {
    let mut env = Env::new();
    env.seed_global_constants();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();

    writeln!(stdout, "Asili REPL. Andika 'toka' au 'exit' kuondoka. ?mada = msaada (mf. ?hisabati).").map_err(|e| CliError::new(e.to_string(), 1))?;
    loop {
        write!(stdout, "{}", PROMPT).map_err(|e| CliError::new(e.to_string(), 1))?;
        stdout.flush().map_err(|e| CliError::new(e.to_string(), 1))?;

        let Some(Ok(line)) = lines.next() else { break; };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(line, "toka" | "exit" | "quit") {
            break;
        }
        if let Some(topic) = line.strip_prefix('?') {
            let topic = topic.trim();
            if let Err(e) = show_repl_doc(topic, &mut stdout) {
                eprintln!("{}", e);
            }
            continue;
        }

        let (module, body) = match parse_repl_line(line) {
            Ok(m) => m,
            Err(diags) => {
                for d in &diags {
                    eprintln!("{}: {} ({}:{})", d.code, d.message, d.span.as_ref().map(|s| s.line).unwrap_or(0), d.span.as_ref().map(|s| s.column).unwrap_or(0));
                }
                continue;
            }
        };

        match run_block_in_env_with_telemetry(&module, &body, &mut env) {
            Ok((v, peak_depth)) => {
                if !matches!(v, Value::Tupu) {
                    println!("{:?}", v);
                }
                if peak_depth > 0 {
                    println!("  (undani: {})", peak_depth);
                }
            }
            Err(e) => eprintln!("kosa: {}", e),
        }
    }
    Ok(())
}

/// Parse line as block or as "rejesha <expr>". No semantic check — REPL runs in persistent env
/// and undefined/type errors are reported at runtime by the evaluator.
fn parse_repl_line(line: &str) -> Result<(asili_parser::Module, asili_parser::Block), Vec<asili_diagnostics::Diagnostic>> {
    let try_parse = |wrapped: &str| -> Result<(asili_parser::Module, asili_parser::Block), Vec<asili_diagnostics::Diagnostic>> {
        let tokens = tokenize(wrapped)?;
        let module = parse_tokens(&tokens)?;
        let body = module
            .functions
            .iter()
            .find(|f| f.name == REPL_FUNC)
            .map(|f| f.body.clone())
            .expect("repl function");
        Ok((module, body))
    };

    let (module, mut body) = try_parse(&format!("kazi {}() -> Tupu {{ {} }}", REPL_FUNC, line))
        .or_else(|_| try_parse(&format!("kazi {}() -> Tupu {{ rejesha {} }}", REPL_FUNC, line)))?;
    // If the only statement is an expression, treat it as "rejesha <expr>" so we print the value.
    if body.statements.len() == 1 {
        if let Stmt::Expr { expr, line } = &body.statements[0] {
            body.statements = vec![Stmt::Return {
                value: Some(expr.clone()),
                line: *line,
            }];
        }
    }
    Ok((module, body))
}

/// Print content of docs/repl/{topic}.md if it exists. Path is relative to current dir.
fn show_repl_doc(topic: &str, out: &mut impl Write) -> Result<(), String> {
    if topic.is_empty() || topic.contains('/') || topic.contains('\\') {
        return Err("jina la mada si sahihi".to_string());
    }
    let path = Path::new("docs").join("repl").join(format!("{}.md", topic));
    let content = std::fs::read_to_string(&path).map_err(|e| {
        if path.exists() {
            e.to_string()
        } else {
            format!("hakuna mada '{}' (tazama docs/repl/)", topic)
        }
    })?;
    writeln!(out, "{}", content).map_err(|e| e.to_string())?;
    Ok(())
}
