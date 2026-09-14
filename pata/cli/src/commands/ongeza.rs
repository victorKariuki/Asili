use super::{CliError, CliResult};
use crate::pipeline::project::{
    load_project_config, update_dependency, validate_dep_name, validate_semver_like, write_lockfile,
};
use std::path::Path;

// Contract: ../../commands/ongeza.md
//
// `--git <url>` sources fetch for real: `pata_package::fetch_git` clones the repo into
// `.asili/packages/<lib>/`, strips `.git/`, and returns a real SHA-256 content hash over the
// fetched tree — written into pata.lock as that dependency's checksum, overwriting whatever
// `write_lockfile`'s normal (non-fetching) resolve path produced. Plain version dependencies
// (no `--git`) still only write to pata.toml/pata.lock with no fetch — there is no registry
// backend yet to fetch them from (see docs/design/pata-production-readiness.md item 3 /
// pata-implementation-spec.md Section 16).
pub fn run(args: &[String]) -> CliResult {
    let (lib, version, git_url, branch) = parse_args(args)?;
    validate_dep_name(&lib)?;
    validate_semver_like(&version)?;

    let root = Path::new(".");
    update_dependency(root, &lib, &version)?;

    if let Some(url) = &git_url {
        let paths = pata_package::Paths::new(root);
        let dest = paths.package_path(&lib);
        let checksum = pata_package::fetch_git(url, branch.as_deref(), &dest)
            .map_err(|e| CliError::new(format!("imeshindwa kupata '{lib}' kutoka {url}: {e}"), 1))?;

        // fetch_git succeeds before write_lockfile's own resolve pass runs, so the freshly
        // fetched checksum is available to overwrite whatever that pass computed for this one
        // dependency (a name/version-string placeholder hash, since Resolver::resolve has no
        // fetched content to hash on a fresh `pata ongeza --git` run).
        let cfg = load_project_config(root)?;
        write_lockfile(root, &cfg)?;
        overwrite_locked_checksum(root, &lib, &checksum, "git")?;
    } else {
        let cfg = load_project_config(root)?;
        write_lockfile(root, &cfg)?;
    }

    println!("imekamilika: tegemezi '{lib}' = '{version}'");
    Ok(())
}

/// Patch a single dependency's checksum/source in the just-written pata.lock with the real
/// fetched content hash. Separate from `write_lockfile`'s own resolve pass rather than
/// threading the fetched checksum through `Resolver::resolve` itself — the resolver has no
/// concept of "a fetch just happened" and giving it one is Section 7/9's job (real constraint
/// solving against a registry/vendor index), not this command's.
fn overwrite_locked_checksum(root: &Path, lib: &str, checksum: &str, source: &str) -> CliResult {
    let lock_path = root.join("pata.lock");
    let mut lock = pata_package::LockFile::load(&lock_path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", lock_path.display()), 1))?;
    if let Some(locked) = lock.dependencies.get_mut(lib) {
        locked.checksum = checksum.to_string();
        locked.source = source.to_string();
    }
    lock.save(&lock_path)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", lock_path.display()), 1))
}

fn parse_args(args: &[String]) -> Result<(String, String, Option<String>, Option<String>), CliError> {
    let Some(lib) = args.first() else {
        return Err(CliError::new(
            "matumizi: pata ongeza <lib> [--toleo <semver>] [--git <url>] [--tawi <jina>]",
            2,
        ));
    };

    let mut version = String::from("^0.1");
    let mut git_url: Option<String> = None;
    let mut branch: Option<String> = None;
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
            "--git" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--git inahitaji URL", 2));
                };
                git_url = Some(v.clone());
                i += 2;
            }
            "--tawi" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--tawi inahitaji jina", 2));
                };
                branch = Some(v.clone());
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

    Ok((lib.clone(), version, git_url, branch))
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
        // pata.lock is written via pata_package::LockFile (real TOML, not the old flat
        // `name = "version"` line format) — the dependency name is a table header.
        assert!(lock.contains("[dependencies.hisabati]"));
        assert!(lock.contains("version = \"^1.2\""));

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    /// Real end-to-end fetch test: `git init`s a source repo on disk (no network — a `file://`
    /// clone of a local path is exactly as real a `git2::RepoBuilder::clone` codepath as a
    /// remote URL, just without hitting the network) with one commit, runs `pata ongeza --git`
    /// against it, and asserts the vendored source actually landed on disk with a real content
    /// hash in pata.lock — not the old `sha256("{name}@{version}")` placeholder.
    #[test]
    fn ongeza_git_fetches_real_source_and_writes_content_hash() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");

        let source_repo = temp_dir_named("ongeza-git-source");
        fs::write(source_repo.join("mtu.as"), "kazi jina() -> Neno { rejesha \"mtu\" }").expect("write source file");
        run_git(&source_repo, &["init", "-q"]);
        run_git(&source_repo, &["config", "user.email", "test@example.com"]);
        run_git(&source_repo, &["config", "user.name", "Test"]);
        run_git(&source_repo, &["add", "-A"]);
        run_git(&source_repo, &["commit", "-q", "-m", "initial"]);

        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        let git_url = format!("file://{}", source_repo.display());
        run(&["mtu".into(), "--git".into(), git_url]).expect("ongeza --git ok");

        // Real content landed on disk under .asili/packages/mtu/, .git/ metadata stripped.
        let vendored = root.join(".asili/packages/mtu/mtu.as");
        assert!(vendored.is_file(), "vendored source file should exist at {}", vendored.display());
        let content = fs::read_to_string(&vendored).expect("read vendored file");
        assert!(content.contains("rejesha \"mtu\""));
        assert!(!root.join(".asili/packages/mtu/.git").exists(), ".git metadata must be stripped from vendored copy");

        // pata.lock's checksum is a real SHA-256 hex digest (64 hex chars) over the fetched
        // tree, not the old 16-hex-char sha256("{name}@{version}") name-string placeholder.
        let lock = fs::read_to_string(root.join("pata.lock")).expect("pata.lock");
        assert!(lock.contains("[dependencies.mtu]"));
        assert!(lock.contains("source = \"git\""));
        let checksum_line = lock
            .lines()
            .find(|l| l.trim_start().starts_with("checksum"))
            .expect("checksum line present");
        let checksum = checksum_line.split('"').nth(1).expect("quoted checksum value");
        assert_eq!(checksum.len(), 64, "expected a full SHA-256 hex digest, got: {checksum}");

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&source_repo);
    }

    #[test]
    fn ongeza_git_with_bad_url_fails_cleanly() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        let result = run(&["haipo".into(), "--git".into(), "file:///nonexistent/path/at/all".into()]);
        assert!(result.is_err(), "fetching a nonexistent git source must fail, not silently succeed");

        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    fn run_git(dir: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git command should spawn");
        assert!(status.success(), "git {:?} failed in {}", args, dir.display());
    }

    fn temp_dir_named(prefix: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-{prefix}-{stamp}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
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
