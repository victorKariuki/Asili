use super::{CliError, CliResult};
use crate::pipeline::compile::compile_project;
use crate::pipeline::format::{check_or_write, collect_asili_files};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;
use std::fs;
use std::path::Path;

// Contract: ../../commands/thibitisha.md
pub fn run(_args: &[String]) -> CliResult {
    compile_project(Path::new("."))?;
    enforce_docs(Path::new("."))?;

    let files = collect_asili_files(Path::new("."))?;
    let (_, changed) = check_or_write(&files, true)?;
    if changed > 0 {
        return Err(CliError::new(
            "nadhifu check imefeli: tumia `pata nadhifu` kwanza",
            1,
        ));
    }

    println!("thibitisha: sawa");
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
                    if ln.starts_with("umma kazi") || ln.starts_with("umma umbo") {
                        if i == 0 || !lines[i - 1].trim().starts_with("///") {
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
                        "nyaraka zimekosekana kwa umbo ya umma '{}' kwenye {}:{}",
                        s.name,
                        file.display(),
                        s.line
                    ),
                    1,
                ));
            }
        }
        for t in &module.traits {
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
