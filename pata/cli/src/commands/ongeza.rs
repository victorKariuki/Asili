use super::{CliError, CliResult};
use crate::pipeline::project::{
    load_project_config, update_dependency, validate_dep_name, validate_semver_like, write_lockfile,
};
use std::path::Path;

// Contract: ../../commands/ongeza.md
//
// TODO: `pata ongeza` writes the dependency to pata.toml and generates pata.lock, but does NOT
// actually download or resolve the package. There is no package registry, resolver, or caching layer.
// To implement: define a registry URL (or local path convention), fetch the package manifest,
// resolve version constraints, download the source, and extract it to a vendor/ or cache directory.
pub fn run(args: &[String]) -> CliResult {
    let (lib, version) = parse_args(args)?;
    validate_dep_name(&lib)?;
    validate_semver_like(&version)?;

    let root = Path::new(".");
    update_dependency(root, &lib, &version)?;
    let cfg = load_project_config(root)?;
    write_lockfile(root, &cfg)?;

    println!("imekamilika: tegemezi '{lib}' = '{version}'");
    Ok(())
}

fn parse_args(args: &[String]) -> Result<(String, String), CliError> {
    let Some(lib) = args.first() else {
        return Err(CliError::new(
            "matumizi: pata ongeza <lib> [--toleo <semver>]",
            2,
        ));
    };

    let mut version = String::from("^0.1");
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--toleo" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--toleo inahitaji thamani", 2));
                };
                version = v.clone();
                i += 2;
            }
            other => {
                return Err(CliError::new(
                    format!("hoja isiyotambuliwa kwenye ongeza: {other}"),
                    2,
                ));
            }
        }
    }

    Ok((lib.clone(), version))
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn updates_manifest_and_lock() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        run(&["hisabati".into(), "--toleo".into(), "^1.2".into()]).expect("ongeza ok");
        let toml = fs::read_to_string("pata.toml").expect("pata.toml");
        let lock = fs::read_to_string("pata.lock").expect("pata.lock");
        assert!(toml.contains("hisabati = \"^1.2\""));
        assert!(lock.contains("hisabati = \"^1.2\""));

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    fn temp_project() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-ongeza-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("write manifest");
        fs::write(dir.join("src/kuu.as"), "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }").expect("src");
        dir
    }
}
