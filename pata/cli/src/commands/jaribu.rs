use super::{CliError, CliResult};
use crate::pipeline::compile::{list_project_tests, run_project_tests};
use std::path::Path;

// Exit codes: 0 = all tests passed; 1 = one or more tests failed; 2 = usage or config error.
// Contract: ../../commands/jaribu.md
pub fn run(args: &[String]) -> CliResult {
    let (filter, fail_fast, list_only) = parse_args(args)?;

    if list_only {
        let names = list_project_tests(Path::new("."))?;
        let names: Vec<&str> = match &filter {
            Some(f) => names.iter().filter(|n| n.contains(f.as_str())).map(String::as_str).collect(),
            None => names.iter().map(String::as_str).collect(),
        };
        for n in names {
            println!("{n}");
        }
        return Ok(());
    }

    let results = run_project_tests(Path::new("."), filter.as_deref(), fail_fast)?;

    let mut passed = 0usize;
    let mut failed = 0usize;
    for r in &results {
        if r.passed {
            passed += 1;
            println!("[SAWA] {}", r.name);
        } else {
            failed += 1;
            println!("[KOSA] {} - {}", r.name, r.message);
        }
    }

    println!("jumla: {} | sawa: {} | kosa: {}", results.len(), passed, failed);
    if failed > 0 {
        println!("majaribio {} yameshindwa", failed);
        return Err(CliError::new("baadhi ya majaribio yameshindwa", 1));
    }
    println!("majaribio yote yamefaulu");
    Ok(())
}

fn parse_args(args: &[String]) -> Result<(Option<String>, bool, bool), CliError> {
    let mut filter = None;
    let mut fail_fast = false;
    let mut list_only = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--filter" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--filter inahitaji pattern", 2));
                };
                filter = Some(v.clone());
                i += 2;
            }
            "--fail-fast" => {
                fail_fast = true;
                i += 1;
            }
            "--list" => {
                list_only = true;
                i += 1;
            }
            other => {
                return Err(CliError::new(
                    format!("hoja isiyotambuliwa kwenye jaribu: {other}"),
                    2,
                ));
            }
        }
    }
    Ok((filter, fail_fast, list_only))
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fails_when_test_panics() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&[]).expect_err("jaribu should fail");
        assert_eq!(err.exit_code, 1);

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn list_exits_zero_and_discovers_tests() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        let result = run(&["--orodha".to_string()]);
        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(root);

        result.expect("jaribu --list should succeed");
    }

    fn temp_project() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-jaribu-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n#[jaribio]\nkazi t_fail() -> Tupu { paparika(\"x\") }",
        )
        .expect("src");
        dir
    }
}
