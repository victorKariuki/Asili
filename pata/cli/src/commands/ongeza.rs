use super::{CliError, CliResult};
use crate::pipeline::project::{
    load_project_config, update_dependency, update_dependency_git, validate_dep_name,
    validate_semver_like, write_lockfile,
};
use std::path::Path;

// Contract: ../../commands/ongeza.md
//
// `--git <url>` sources fetch for real: `pata_package::fetch_git` clones the repo into
// `.asili/packages/<lib>/`, strips `.git/`, and returns a real SHA-256 content hash over the
// fetched tree. The manifest records the git URL (via `update_dependency_git`, writing `{ git =
// "...", version = "..." }` instead of a bare version string) so `pata_package::Resolver::
// resolve`'s real constraint-solving pass (not a placeholder) correctly treats this as a git
// dependency on every subsequent resolve, matched against the `.pata-version` marker this
// command writes right after fetching. Plain version dependencies (no `--git`) resolve against
// the local registry index at `.asili/registry/` instead — see `pata_package::resolver`.
pub fn run(args: &[String]) -> CliResult {
    let (lib, version, git_url, branch) = parse_args(args)?;
    validate_dep_name(&lib)?;
    validate_semver_like(&version)?;

    let root = Path::new(".");

    if let Some(url) = &git_url {
        update_dependency_git(root, &lib, &version, url)?;

        let paths = pata_package::Paths::new(root);
        let dest = paths.package_path(&lib);
        let checksum = pata_package::fetch_git(url, branch.as_deref(), &dest)
            .map_err(|e| CliError::new(format!("imeshindwa kupata '{lib}' kutoka {url}: {e}"), 1))?;

        // The resolver's git-dependency path reads this marker to know which exact version was
        // fetched (there's no registry to list multiple versions of a git source against) —
        // without it, every subsequent resolve (including write_lockfile below, in the very
        // same run) would find nothing vendored and fail. The marker must be a full,
        // `semver::Version`-parseable `major.minor.patch` string — `--toleo`'s given constraint
        // (e.g. `^0.1`, this command's own default) is a *range*, not necessarily one, so a
        // constraint that doesn't already parse as an exact version is padded out to one
        // (`^0.1` -> `0.1.0`) rather than written verbatim and left to fail deep inside the next
        // resolve with a confusing "invalid semver in marker file" error.
        let marker_version = exact_version_for_marker(&version);
        std::fs::write(dest.join(".pata-version"), &marker_version)
            .map_err(|e| CliError::new(format!("imeshindwa kuandika alama ya toleo kwa '{lib}': {e}"), 1))?;

        let cfg = load_project_config(root)?;
        write_lockfile(root, &cfg)?;
        // write_lockfile's own resolve pass now finds the marker and produces a real git-sourced
        // checksum via Resolver::resolve itself — but re-verify/overwrite with the checksum this
        // command just computed directly from the freshly fetched tree, in case a differently
        // shaped marker/constraint interaction inside the resolver ever diverges from it.
        overwrite_locked_checksum(root, &lib, &checksum, "git")?;
    } else {
        update_dependency(root, &lib, &version)?;
        let cfg = load_project_config(root)?;
        write_lockfile(root, &cfg)?;
    }

    println!("imekamilika: tegemezi '{lib}' = '{version}'");
    Ok(())
}

/// Patch a single dependency's checksum/source in the just-written pata.lock with the real
/// fetched content hash — a final, direct-from-the-fetch-itself verification pass on top of
/// `Resolver::resolve`'s own (also-real, since the resolver rewrite) git-dependency checksum.
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

/// Turn a version *constraint* (`^0.1`, `~2.3`, a bare `1.2.3`, etc.) into a concrete
/// `major.minor.patch` string suitable for `.pata-version` marker files, which
/// `pata_package::resolver::resolve_git_dependency` parses with `semver::Version::parse` (which
/// requires all three components — `^0.1` alone is not a valid `Version`, only a valid
/// `VersionReq`). Strips any leading constraint operator, then pads missing components with `0`.
fn exact_version_for_marker(constraint: &str) -> String {
    let bare = constraint.trim_start_matches(['^', '~', '>', '<', '=']).trim();
    let mut parts: Vec<&str> = bare.split('.').collect();
    while parts.len() < 3 {
        parts.push("0");
    }
    parts.truncate(3);
    parts.join(".")
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
    use super::{exact_version_for_marker, run};
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn exact_version_for_marker_pads_short_constraints() {
        assert_eq!(exact_version_for_marker("^0.1"), "0.1.0");
        assert_eq!(exact_version_for_marker("~2.3"), "2.3.0");
        assert_eq!(exact_version_for_marker("1"), "1.0.0");
    }

    #[test]
    fn exact_version_for_marker_passes_through_full_versions() {
        assert_eq!(exact_version_for_marker("1.2.3"), "1.2.3");
        assert_eq!(exact_version_for_marker(">=1.2.3"), "1.2.3");
    }

    #[test]
    fn updates_manifest_and_lock() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        // A bare-version (non-`--git`) `ongeza` now resolves against the real local registry
        // index (Resolver::resolve does real constraint solving, not a placeholder) — publish a
        // real 1.2.0 entry first so this exercises the actual end-to-end path instead of relying
        // on the old resolver's "accept anything" behavior.
        let published = root.join("published-hisabati");
        fs::create_dir_all(published.join("src")).expect("mkdir published");
        fs::write(published.join("src/hisabati.as"), "umma kazi jumlisha(a: Namba, b: Namba) -> Namba { rejesha a + b }\n")
            .expect("write published source");
        let mut registry = pata_package::LocalRegistry::at(root.join(".asili/registry")).expect("open registry");
        registry.publish(pata_package::RegistryEntry {
            name: "hisabati".to_string(),
            vers: "1.2.0".to_string(),
            deps: vec![],
            yanked: None,
            source: pata_package::RegistrySource::Path {
                path: published.to_string_lossy().to_string(),
            },
        }).expect("publish hisabati 1.2.0");

        run(&["hisabati".into(), "--toleo".into(), "^1.2".into()]).expect("ongeza ok");
        let toml = fs::read_to_string("pata.toml").expect("pata.toml");
        let lock = fs::read_to_string("pata.lock").expect("pata.lock");
        assert!(toml.contains("hisabati = \"^1.2\""));
        // pata.lock is written via pata_package::LockFile (real TOML, not the old flat
        // `name = "version"` line format) — the dependency name is a table header.
        assert!(lock.contains("[dependencies.hisabati]"));
        // The resolved concrete version (from the real registry match), not the constraint
        // string itself — proves real resolution happened, not a verbatim-lock placeholder.
        assert!(lock.contains("version = \"1.2.0\""));
        assert!(lock.contains("source = \"registry\""));

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
