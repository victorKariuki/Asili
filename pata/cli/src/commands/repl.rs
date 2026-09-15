//! REPL: read-eval-print loop with persistent env. Use `?topic` to show docs from
//! docs/repl/{lang}/{topic}.md. `?lugha en`/`?lugha sw` switches the help language
//! for the rest of the session (default: sw).

use super::{CliError, CliResult};
use asili_evaluator::{run_block_in_env_with_telemetry, Env, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, Stmt};
use std::io::{self, BufRead, Write};
use std::path::Path;
use pulldown_cmark::{Parser, Event};

const REPL_FUNC: &str = "__repl__";
const PROMPT: &str = "> ";

pub fn run(_args: &[String]) -> CliResult {
    let mut env = Env::new();
    env.seed_global_constants();
    let mut lang = "sw".to_string();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();

    writeln!(stdout, "Asili REPL. Andika 'toka' au 'exit' kuondoka. ?mada = msaada (mf. ?hisabati). ?lugha en/sw = badilisha lugha ya msaada.").map_err(|e| CliError::new(e.to_string(), 1))?;
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
            if let Some(requested) = topic.strip_prefix("lugha") {
                let requested = requested.trim();
                match requested {
                    "en" | "sw" => {
                        lang = requested.to_string();
                        println!("lugha ya msaada: {}", lang);
                    }
                    _ => eprintln!("tumia ?lugha en au ?lugha sw"),
                }
                continue;
            }
            if let Err(e) = show_repl_doc(topic, &lang, &mut stdout) {
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

/// Parse line as block or as "rejesha `<expr>`". No semantic check — REPL runs in persistent env
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

/// Print and render content of docs/repl/{lang}/{topic}.md if it exists. Path is relative to current dir.
fn show_repl_doc(topic: &str, lang: &str, out: &mut impl Write) -> Result<(), String> {
    if topic.is_empty() || topic.contains('/') || topic.contains('\\') {
        return Err("jina la mada si sahihi".to_string());
    }
    let path = Path::new("docs").join("repl").join(lang).join(format!("{}.md", topic));
    let content = std::fs::read_to_string(&path).map_err(|e| {
        if path.exists() {
            e.to_string()
        } else {
            format!("hakuna mada '{}' (tazama docs/repl/{})", topic, lang)
        }
    })?;

    let parser = Parser::new(&content);
    let rendered = render_markdown_to_terminal(parser);
    writeln!(out, "{}", rendered).map_err(|e| e.to_string())?;
    Ok(())
}

fn render_markdown_to_terminal(parser: Parser) -> String {
    let mut output = String::new();
    let mut in_table = false;

    for event in parser {
        match event {
            Event::Start(tag) => {
                match tag {
                    pulldown_cmark::Tag::Heading { level, .. } => {
                        output.push_str("\x1b[1m"); // bold
                        if matches!(level, pulldown_cmark::HeadingLevel::H1 | pulldown_cmark::HeadingLevel::H2) {
                            output.push_str("\x1b[36m"); // cyan
                        }
                    }
                    pulldown_cmark::Tag::CodeBlock(_) => {
                        output.push_str("\x1b[90m"); // dark gray
                    }
                    pulldown_cmark::Tag::Table(_) => {
                        in_table = true;
                    }
                    pulldown_cmark::Tag::Emphasis => {
                        output.push_str("\x1b[3m"); // italic
                    }
                    pulldown_cmark::Tag::Strong => {
                        output.push_str("\x1b[1m"); // bold
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                match tag {
                    pulldown_cmark::TagEnd::Heading(_) => {
                        output.push_str("\x1b[0m\n"); // reset + newline
                    }
                    pulldown_cmark::TagEnd::CodeBlock => {
                        output.push_str("\x1b[0m\n"); // reset + newline
                    }
                    pulldown_cmark::TagEnd::Paragraph => {
                        if !in_table {
                            output.push('\n');
                        }
                    }
                    pulldown_cmark::TagEnd::Table => {
                        in_table = false;
                    }
                    pulldown_cmark::TagEnd::Emphasis | pulldown_cmark::TagEnd::Strong => {
                        output.push_str("\x1b[0m"); // reset
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                output.push_str(&text);
            }
            Event::Code(code) => {
                output.push_str("\x1b[92m"); // green
                output.push('`');
                output.push_str(&code);
                output.push('`');
                output.push_str("\x1b[0m"); // reset
            }
            Event::SoftBreak | Event::HardBreak => {
                output.push('\n');
            }
            _ => {}
        }
    }
    output
}
