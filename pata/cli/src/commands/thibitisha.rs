use super::{CliError, CliResult};
use crate::pipeline::compile::compile_project;
use crate::pipeline::format::{check_or_write, collect_asili_files};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, Module};
use std::fs;
use std::path::Path;

// Contract: ../../commands/thibitisha.md
//
// TODO: thibitisha currently checks three things: public-item doc coverage, formatting
// compliance, and (opt-in via --kiwango-cha-jaribio) test coverage. Missing checks from the
// spec:
//   - Type stability: public function signatures must not change in a breaking way between versions
//   - ABI compatibility: exported kiunganishi functions must match declared C signatures
//   - Trait completeness: every sifa listed in [tegemezi] must be fully implemented
pub fn run(args: &[String]) -> CliResult {
    let threshold = parse_args(args)?;

    let output = compile_project(Path::new("."), None)?;
    enforce_docs(Path::new("."))?;

    let files = collect_asili_files(Path::new("."))?;
    let (_, changed) = check_or_write(&files, true)?;
    if changed > 0 {
        return Err(CliError::new(
            "mafaili hayajafuata muundo sahihi: tumia `pata nadhifu` kwanza",
            1,
        ));
    }

    if let Some(threshold) = threshold {
        enforce_test_coverage(&output.module, threshold)?;
    }

    println!("thibitisha: sawa");
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Option<f64>, CliError> {
    let mut threshold = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--kiwango-cha-jaribio" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--kiwango-cha-jaribio inahitaji thamani (0-100)", 2));
                };
                let parsed: f64 = v.parse().map_err(|_| {
                    CliError::new(format!("--kiwango-cha-jaribio inahitaji namba (0-100), si: {v}"), 2)
                })?;
                if !(0.0..=100.0).contains(&parsed) {
                    return Err(CliError::new(
                        format!("--kiwango-cha-jaribio inahitaji thamani kati ya 0 na 100, si: {parsed}"),
                        2,
                    ));
                }
                threshold = Some(parsed);
                i += 2;
            }
            other => {
                return Err(CliError::new(
                    format!("hoja isiyotambuliwa kwenye thibitisha: {other}"),
                    2,
                ));
            }
        }
    }
    Ok(threshold)
}

/// Ratio of public `kazi` with a corresponding `#[jaribio]` test to total public `kazi`,
/// checked against `threshold` percent. A public function counts as "covered" if a test
/// function with a matching name convention (`jaribio_<name>` or simply any #[jaribio]
/// function, since Asili has no call-graph/coverage instrumentation) exists — kept
/// deliberately simple: presence of at least `threshold`% as many #[jaribio] functions as
/// public kazi, not per-function attribution.
fn enforce_test_coverage(module: &Module, threshold: f64) -> CliResult {
    let public_count = module.functions.iter().filter(|f| f.is_public).count();
    if public_count == 0 {
        return Ok(());
    }
    let test_count = module.functions.iter().filter(|f| f.is_test).count();
    let ratio = (test_count as f64 / public_count as f64) * 100.0;
    if ratio < threshold {
        return Err(CliError::new(
            format!(
                "kiwango cha majaribio {:.1}% ni chini ya kiwango kinachohitajika {:.1}% (majaribio {test_count} kwa kazi za umma {public_count})",
                ratio, threshold
            ),
            1,
        ));
    }
    Ok(())
}

fn has_doc_before(lines: &[&str], line_1based: usize) -> bool {
    let mut i = line_1based.saturating_sub(2) as i32;
    while i >= 0 {
        let ln = lines[i as usize].trim();
        if ln.is_empty() {
            i -= 1;
            continue;
        }
        return ln.starts_with("///");
    }
    false
}

fn enforce_docs(root: &Path) -> CliResult {
    let files = collect_asili_files(root)?;
    for file in files {
        let src = fs::read_to_string(&file)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", file.display()), 1))?;
        let lines: Vec<&str> = src.lines().collect();
        let toks = match tokenize(&src) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let module = match parse_tokens(&toks) {
            Ok(m) => m,
            Err(_) => {
                for i in 0..lines.len() {
                    let ln = lines[i].trim();
                    if (ln.starts_with("umma kazi") || ln.starts_with("umma umbo")) && (i == 0 || !lines[i - 1].trim().starts_with("///")) {
                        return Err(CliError::new(
                            format!(
                                "nyaraka zimekosekana kwa item ya umma kwenye {}:{}",
                                file.display(),
                                i + 1
                            ),
                            1,
                        ));
                    }
                }
                continue;
            }
        };
        for f in &module.functions {
            if f.is_public && !has_doc_before(&lines, f.line) {
                return Err(CliError::new(
                    format!(
                        "nyaraka zimekosekana kwa kazi ya umma '{}' kwenye {}:{}",
                        f.name,
                        file.display(),
                        f.line
                    ),
                    1,
                ));
            }
        }
        for s in &module.structs {
            if s.is_public && !has_doc_before(&lines, s.line) {
                return Err(CliError::new(
                    format!(
                        "nyaraka zimekosekana kwa umbo la umma '{}' kwenye {}:{}",
                        s.name,
                        file.display(),
                        s.line
                    ),
                    1,
                ));
            }
        }
        for t in &module.traits {
            // Skip language-seeded traits (Inasomeka, Inandikika — see standard_traits() in
            // core/parser/src/parse.rs): they're hardcoded Rust literals with no real source
            // location (line 0), not something any project file could ever add a doc comment
            // above, and they appear in every module's `traits` list regardless of file content.
            if t.line == 0 {
                continue;
            }
            if t.is_public && !has_doc_before(&lines, t.line) {
                return Err(CliError::new(
                    format!(
                        "nyaraka zimekosekana kwa sifa ya umma '{}' kwenye {}:{}",
                        t.name,
                        file.display(),
                        t.line
                    ),
                    1,
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fails_when_public_item_has_no_docs() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&[]).expect_err("thibitisha should fail");
        assert_eq!(err.exit_code, 1);

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn passes_when_no_public_items_or_docs_ok() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_no_public();
        std::env::set_current_dir(&root).expect("chdir");

        let result = run(&[]);
        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
        result.expect("thibitisha should pass when there are no public items to document");
    }

    #[test]
    fn test_coverage_threshold_fails_when_below_target() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_uncovered_public_fn();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&["--kiwango-cha-jaribio".to_string(), "100".to_string()])
            .expect_err("thibitisha should fail below coverage threshold");
        assert_eq!(err.exit_code, 1);
        assert!(err.message.contains("kiwango cha majaribio"));

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_coverage_threshold_not_enforced_by_default() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_uncovered_public_fn();
        std::env::set_current_dir(&root).expect("chdir");

        let result = run(&[]);
        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
        result.expect("thibitisha should not enforce coverage without the flag");
    }

    fn temp_project_uncovered_public_fn() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-cov-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n/// Jumlisha namba mbili.\numma kazi jumlisha(a: Namba, b: Namba) -> Namba { rejesha a + b }\n",
        )
        .expect("src");
        dir
    }

    fn temp_project_no_public() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-ok-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n",
        )
        .expect("src");
        dir
    }

    fn temp_project() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\numma kazi wazi() -> Tupu { }",
        )
        .expect("src");
        dir
    }
}
