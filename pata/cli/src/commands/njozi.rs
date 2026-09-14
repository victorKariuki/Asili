use super::{CliError, CliResult};
use std::fs;
use std::path::{Path, PathBuf};

// Contract: ../../commands/njozi.md
pub fn run(args: &[String]) -> CliResult {
    if args.len() > 2 {
        return Err(CliError::new(
            "matumizi: pata njozi [jina_la_mradi] [njia]",
            2,
        ));
    }

    let (project_name, destination) = parse_inputs(args)?;
    create_scaffold(&project_name, &destination)?;
    println!(
        "imekamilika: mradi '{}' umeundwa katika {}",
        project_name,
        destination.display()
    );

    Ok(())
}

fn parse_inputs(args: &[String]) -> Result<(String, PathBuf), CliError> {
    match args {
        [] => {
            let project_name = String::from("asili-app");
            Ok((project_name.clone(), PathBuf::from(project_name)))
        }
        [name] => {
            validate_project_name(name)?;
            Ok((name.clone(), PathBuf::from(name)))
        }
        [name, dest] => {
            validate_project_name(name)?;
            Ok((name.clone(), PathBuf::from(dest)))
        }
        _ => Err(CliError::new(
            "matumizi: pata njozi [jina_la_mradi] [njia]",
            2,
        )),
    }
}

fn validate_project_name(name: &str) -> Result<(), CliError> {
    if name.trim().is_empty() {
        return Err(CliError::new("jina_la_mradi haliwezi kuwa tupu", 2));
    }
    Ok(())
}

fn create_scaffold(project_name: &str, destination: &Path) -> CliResult {
    ensure_destination_ready(destination)?;

    fs::create_dir_all(destination.join("src"))
        .map_err(|err| CliError::new(format!("imeshindwa kuunda src/: {err}"), 1))?;
    fs::create_dir_all(destination.join("kilele"))
        .map_err(|err| CliError::new(format!("imeshindwa kuunda kilele/: {err}"), 1))?;

    write_file(
        &destination.join("pata.toml"),
        &format!(
            "[jumla]\njina = \"{project_name}\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n"
        ),
    )?;
    write_file(
        &destination.join("src/kuu.as"),
        "leta matumizi\n\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    ikiwa hoja.urefu() > 1 {\n        chapisha(\"Asili scaffold iko tayari.\")\n    } vinginevyo {\n        chapisha(\"Habari Asili!\")\n    }\n}\n",
    )?;
    write_file(
        &destination.join(".gitignore"),
        "kilele/*\n!kilele/.gitkeep\n\n*.asb\n*.asm\n",
    )?;
    write_file(&destination.join("kilele/.gitkeep"), "")?;

    Ok(())
}

fn ensure_destination_ready(destination: &Path) -> CliResult {
    if destination.exists() {
        let mut entries = fs::read_dir(destination).map_err(|err| {
            CliError::new(
                format!("imeshindwa kusoma eneo la mradi {}: {err}", destination.display()),
                1,
            )
        })?;
        if entries.next().is_some() {
            return Err(CliError::new(
                format!(
                    "eneo la mradi tayari lina mafaili: {}",
                    destination.display()
                ),
                2,
            ));
        }
    } else {
        fs::create_dir_all(destination).map_err(|err| {
            CliError::new(
                format!("imeshindwa kuunda eneo la mradi {}: {err}", destination.display()),
                1,
            )
        })?;
    }

    Ok(())
}

fn write_file(path: &Path, content: &str) -> CliResult {
    fs::write(path, content)
        .map_err(|err| CliError::new(format!("imeshindwa kuandika {}: {err}", path.display()), 1))
}

#[cfg(test)]
mod tests {
    use super::{parse_inputs, run};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_defaults_to_asili_app() {
        let args: Vec<String> = vec![];
        let (name, path) = parse_inputs(&args).expect("should parse defaults");
        assert_eq!(name, "asili-app");
        assert_eq!(path.to_string_lossy(), "asili-app");
    }

    #[test]
    fn njozi_creates_expected_files() {
        let temp = unique_temp_dir("njozi-ok");
        let project_path = temp.join("mradi");
        let args = vec![
            String::from("mradi"),
            project_path.to_string_lossy().to_string(),
        ];

        run(&args).expect("njozi should succeed");

        assert!(project_path.join("pata.toml").exists());
        assert!(project_path.join("src/kuu.as").exists());
        assert!(project_path.join(".gitignore").exists());
        assert!(project_path.join("kilele/.gitkeep").exists());

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn njozi_rejects_non_empty_destination() {
        let temp = unique_temp_dir("njozi-non-empty");
        let project_path = temp.join("mradi");
        fs::create_dir_all(&project_path).expect("mkdir");
        fs::write(project_path.join("already.txt"), "x").expect("write sentinel");

        let args = vec![
            String::from("mradi"),
            project_path.to_string_lossy().to_string(),
        ];
        let err = run(&args).expect_err("must fail on non-empty destination");
        assert_eq!(err.exit_code, 2);

        let _ = fs::remove_dir_all(temp);
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-cli-{prefix}-{stamp}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
