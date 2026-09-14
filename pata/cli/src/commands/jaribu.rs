use super::{CliError, CliResult};
use crate::pipeline::compile::{list_project_tests, run_project_tests_parallel};
use std::path::Path;

// Exit codes: 0 = all tests passed; 1 = one or more tests failed; 2 = usage or config error.
// Contract: ../../commands/jaribu.md
pub fn run(args: &[String]) -> CliResult {
    let (filter, fail_fast, list_only, json, num_threads) = parse_args(args)?;

    if list_only {
        let names = list_project_tests(Path::new("."))?;
        let names: Vec<&str> = match &filter {
            Some(f) => names.iter().filter(|n| n.contains(f.as_str())).map(String::as_str).collect(),
            None => names.iter().map(String::as_str).collect(),
        };
        if json {
            let output = serde_json::json!({ "majaribio": names });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        } else {
            for n in names {
                println!("{n}");
            }
        }
        return Ok(());
    }

    let results = run_project_tests_parallel(Path::new("."), filter.as_deref(), fail_fast, num_threads)?;

    let mut passed = 0usize;
    let mut failed = 0usize;
    for r in &results {
        if r.passed {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    if json {
        let output = serde_json::json!({
            "jumla": results.len(),
            "sawa": passed,
            "kosa": failed,
            "majaribio": results.iter().map(|r| serde_json::json!({
                "jina": r.name,
                "sawa": r.passed,
                "ujumbe": if r.passed { serde_json::json!(null) } else { serde_json::json!(r.message) }
            })).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        for r in &results {
            if r.passed {
                println!("[SAWA] {}", r.name);
            } else {
                println!("[KOSA] {} - {}", r.name, r.message);
            }
        }
        println!("jumla: {} | sawa: {} | kosa: {}", results.len(), passed, failed);
    }

    if failed > 0 {
        if !json {
            println!("majaribio {} yameshindwa", failed);
        }
        return Err(CliError::new("baadhi ya majaribio yameshindwa", 1));
    }
    if !json {
        println!("majaribio yote yamefaulu");
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Result<(Option<String>, bool, bool, bool, Option<usize>), CliError> {
    let mut filter = None;
    let mut fail_fast = false;
    let mut list_only = false;
    let mut json = false;
    let mut num_threads = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--chuja" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--chuja inahitaji muundo", 2));
                };
                filter = Some(v.clone());
                i += 2;
            }
            "--simama-haraka" => {
                fail_fast = true;
                i += 1;
            }
            "--orodha" => {
                list_only = true;
                i += 1;
            }
            "--json" => {
                json = true;
                i += 1;
            }
            "--nyuzi-za-jaribio" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--nyuzi-za-jaribio inahitaji namba", 2));
                };
                num_threads = Some(v.parse::<usize>().map_err(|_| {
                    CliError::new(format!("--nyuzi-za-jaribio: '{}' si namba halali", v), 2)
                })?);
                i += 2;
            }
            other => {
                return Err(CliError::new(
                    format!("hoja isiyotambuliwa kwenye jaribu: {other}"),
                    2,
                ));
            }
        }
    }
    Ok((filter, fail_fast, list_only, json, num_threads))
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
