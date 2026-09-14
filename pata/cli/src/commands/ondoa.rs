//! `pata ondoa` — remove a dependency

use crate::commands::CliError;
use crate::pipeline::project::{load_project_config, remove_dependency};
use pata_package::LockFile;
use std::path::Path;

pub type CliResult = Result<(), CliError>;

pub fn run(args: &[String]) -> CliResult {
    if args.is_empty() {
        return Err(CliError::new(
            "matumizi: pata ondoa <jina-la-tegemezi>".to_string(),
            2,
        ));
    }
    let lib_name = &args[0];
    let root = Path::new(".");

    let mut cfg = load_project_config(root)?;
    remove_dependency(&mut cfg, lib_name)?;

    // Write updated pata.lock
    let lock_path = root.join("pata.lock");
    let mut lock = if lock_path.exists() {
        LockFile::load(&lock_path)
            .map_err(|e| CliError::new(format!("imeshindwa kupakia pata.lock: {e}"), 1))?
    } else {
        LockFile::new()
    };

    // Remove from lock too
    if lock.is_locked(lib_name) {
        lock.dependencies.remove(lib_name);
    }
    lock.save(&lock_path)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika pata.lock: {e}"), 1))?;

    println!("tegemezi '{}' imeondolewa", lib_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn removes_an_existing_dependency_from_pata_toml() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_with_dependency();
        std::env::set_current_dir(&root).expect("chdir");

        run(&["hisabati".to_string()]).expect("ondoa ok");

        let toml = fs::read_to_string("pata.toml").expect("pata.toml");
        assert!(!toml.contains("hisabati"), "dependency should be removed from pata.toml, got: {toml}");
        let lock = fs::read_to_string("pata.lock").expect("pata.lock");
        assert!(!lock.contains("[dependencies.hisabati]"), "dependency should be removed from pata.lock, got: {lock}");

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fails_with_exit_code_1_when_removing_a_nonexistent_dependency() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_with_dependency();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&["haipo_kabisa".to_string()]).expect_err("removing a nonexistent dependency should fail");
        assert_eq!(err.exit_code, 1);

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fails_with_exit_code_2_when_no_argument_given() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_with_dependency();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&[]).expect_err("no argument should be a usage error");
        assert_eq!(err.exit_code, 2);

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    /// A project with a real dependency already present in both `pata.toml` and `pata.lock`
    /// (path-based, so no registry/fetch is needed to make the lock entry real) — the fixture
    /// `ondoa` needs to prove removal against, not an empty `[tegemezi]` section.
    fn temp_project_with_dependency() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-ondoa-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::create_dir_all(dir.join("hisabati_pkg/src")).expect("mkdir dep");
        fs::write(
            dir.join("hisabati_pkg/src/hisabati.as"),
            "umma kazi jumlisha(a: Namba, b: Namba) -> Namba { rejesha a + b }\n",
        ).expect("write dep source");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\nhisabati = { path = \"hisabati_pkg\" }\n",
        )
        .expect("write manifest");
        fs::write(dir.join("src/kuu.as"), "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }").expect("src");

        // Write a real pata.lock entry too, matching what `pata ongeza` would have produced --
        // ondoa must clean this up as well, not just pata.toml.
        fs::write(
            dir.join("pata.lock"),
            "version = \"1\"\nlocked_at = \"2026-01-01T00:00:00Z\"\n\n[dependencies.hisabati]\nversion = \"0.0.0\"\nchecksum = \"abc123\"\npath = \"hisabati_pkg\"\nsource = \"path\"\n",
        ).expect("write lock");

        dir
    }
}
