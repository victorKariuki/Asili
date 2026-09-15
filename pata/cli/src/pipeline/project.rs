use crate::commands::CliError;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Re-exported from `pata-core` (the shared resolver crate `pata-cli` and `pata-lsp` both
/// depend on) rather than defined here — kept as a `pub use` so every existing `pata-cli` call
/// site (`Dependency::Version(...)`, `ProjectConfig.dependencies: BTreeMap<String, Dependency>`,
/// etc.) keeps compiling unchanged after the pata-core extraction.
pub use pata_core::Dependency;

#[derive(Clone, Debug)]
pub struct ProjectConfig {
    pub name: String,
    // Parsed from pata.toml but not yet consumed anywhere: `asili_version` isn't checked against
    // the running toolchain (see the TODO on load_project_config below), and `version` stopped
    // feeding the lockfile once write_lockfile moved to pata_package::LockFile, whose schema
    // tracks dependency locks only (like Cargo.lock, not the root project's own version).
    #[allow(dead_code)]
    pub version: String,
    #[allow(dead_code)]
    pub asili_version: String,
    pub entrypoint: PathBuf,
    pub dependencies: BTreeMap<String, Dependency>,
    /// `[jenga] lengo = "..."` — the manifest-declared build target (e.g. "wasm"). `None` means
    /// unset; the CLI defaults to "native" unless `--target` overrides it.
    pub target: Option<String>,
    /// `[eneo-kazi] wanachama = [...]` — relative paths to workspace member project directories,
    /// each with its own `pata.toml`. `None` means this project isn't a workspace root. Unifies
    /// workspace declaration into the same file and Swahili-keyed syntax as everything else in
    /// `pata.toml`, replacing the earlier design of a separate English-keyed `Asili.toml` (see
    /// `find_workspace_root` below and `docs/design/package-manager-design.md`).
    pub eneo_kazi: Option<Vec<String>>,
}

/// Parses `pata.toml` with a real TOML library (`toml::Table`) rather than the earlier
/// hand-rolled line-by-line scanner — the scanner could not represent nested tables at all
/// (needed for `[eneo-kazi]`'s `wanachama` array), and a real parser also rejects genuinely
/// malformed TOML instead of silently skipping unparseable lines. Reads the same Swahili section/
/// key names the old parser did (`[jumla]`, `[chanzo]`, `[tegemezi]`, `[jenga]`), so every
/// existing hand-written `pata.toml` fixture in this codebase's own tests keeps parsing
/// identically — only the parsing mechanism changed, not the file format.
pub fn load_project_config(root: &Path) -> Result<ProjectConfig, CliError> {
    let path = root.join("pata.toml");
    let content = fs::read_to_string(&path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1))?;
    let table: toml::Table = toml::from_str(&content)
        .map_err(|e| CliError::new(format!("hitilafu ya kuchambua {}: {e}", path.display()), 2))?;

    let jumla = table.get("jumla").and_then(|v| v.as_table());
    let name = jumla
        .and_then(|t| t.get("jina"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let version = jumla
        .and_then(|t| t.get("toleo"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let asili_version = jumla
        .and_then(|t| t.get("asili"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let entry = table
        .get("chanzo")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("kuingia"))
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("src/kuu.as"));

    let target = table
        .get("jenga")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("lengo"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let eneo_kazi = table
        .get("eneo-kazi")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("wanachama"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>());

    let mut deps: BTreeMap<String, Dependency> = BTreeMap::new();
    if let Some(tegemezi) = table.get("tegemezi").and_then(|v| v.as_table()) {
        for (key, val) in tegemezi {
            match val {
                toml::Value::String(v) => {
                    deps.insert(key.clone(), Dependency::Version(v.clone()));
                }
                toml::Value::Table(inline) => {
                    let path_val = inline.get("path").and_then(|v| v.as_str());
                    let git_val = inline.get("git").and_then(|v| v.as_str());
                    let version_val = inline.get("version").and_then(|v| v.as_str());
                    if let Some(p) = path_val {
                        deps.insert(key.clone(), Dependency::Path(PathBuf::from(p)));
                    } else if let Some(url) = git_val {
                        deps.insert(key.clone(), Dependency::Git {
                            url: url.to_string(),
                            version: version_val.unwrap_or("*").to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    if name.is_empty() {
        return Err(CliError::new("pata.toml haina [jumla].jina", 2));
    }
    if version.is_empty() {
        return Err(CliError::new("pata.toml haina [jumla].toleo", 2));
    }
    if asili_version.is_empty() {
        return Err(CliError::new("pata.toml haina [jumla].asili", 2));
    }

    // TODO: asili_version is read from pata.toml but never validated or used for compatibility
    // checking. A project declaring `asili = "1.1"` runs fine on any interpreter version with no
    // warning when features from a newer spec are used. Should compare against TOLEO at build time.
    Ok(ProjectConfig {
        name,
        version,
        asili_version,
        entrypoint: root.join(entry),
        dependencies: deps,
        target,
        eneo_kazi,
    })
}

pub fn validate_dep_name(name: &str) -> Result<(), CliError> {
    if name.is_empty() {
        return Err(CliError::new("jina la tegemezi haliwezi kuwa tupu", 2));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(CliError::new(
            "jina la tegemezi lazima litumie herufi/namba/-/_ pekee",
            2,
        ));
    }
    Ok(())
}

pub fn validate_semver_like(v: &str) -> Result<(), CliError> {
    if v.is_empty() {
        return Err(CliError::new("toleo haliwezi kuwa tupu", 2));
    }
    if !v
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || ".^~*<>=".contains(c))
    {
        return Err(CliError::new("toleo lina alama zisizokubalika", 2));
    }
    Ok(())
}

pub fn update_dependency(root: &Path, dep: &str, version: &str) -> Result<(), CliError> {
    update_dependency_entry(root, dep, &dependency_toml_line(dep, version, None))
}

/// Like `update_dependency`, but records a git source too — writes the `{ git = "...", version
/// = "..." }` table form instead of a bare version string, so a later resolve
/// (`pata_package::Resolver::resolve`) correctly treats this as a git dependency (matched
/// against its vendored `.pata-version` marker) rather than a registry one (matched against a
/// local index that has no entry for it).
pub fn update_dependency_git(root: &Path, dep: &str, version: &str, git_url: &str) -> Result<(), CliError> {
    update_dependency_entry(root, dep, &dependency_toml_line(dep, version, Some(git_url)))
}

fn dependency_toml_line(dep: &str, version: &str, git_url: Option<&str>) -> String {
    match git_url {
        Some(url) => format!("{dep} = {{ git = \"{url}\", version = \"{version}\" }}"),
        None => format!("{dep} = \"{version}\""),
    }
}

fn update_dependency_entry(root: &Path, dep: &str, new_line: &str) -> Result<(), CliError> {
    let path = root.join("pata.toml");
    let content = fs::read_to_string(&path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1))?;

    let mut out = Vec::new();
    let mut in_dep = false;
    let mut inserted = false;
    let mut replaced = false;

    for raw in content.lines() {
        let line = raw.to_string();
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_dep && !inserted {
                out.push(new_line.to_string());
                inserted = true;
            }
            in_dep = trimmed == "[tegemezi]";
            out.push(line);
            continue;
        }

        if in_dep {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == dep {
                    out.push(new_line.to_string());
                    replaced = true;
                    inserted = true;
                    continue;
                }
            }
        }

        out.push(line);
    }

    if !content.contains("[tegemezi]") {
        out.push(String::new());
        out.push("[tegemezi]".to_string());
        out.push(new_line.to_string());
    } else if !inserted {
        out.push(new_line.to_string());
    }

    let final_content = format!("{}\n", out.join("\n"));
    fs::write(&path, final_content)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", path.display()), 1))?;

    let action = if replaced { "imesasishwa" } else { "imeongezwa" };
    println!("tegemezi '{dep}' {action}");
    Ok(())
}

pub fn remove_dependency(cfg: &mut ProjectConfig, name: &str) -> Result<(), CliError> {
    if !cfg.dependencies.contains_key(name) {
        return Err(CliError::new(
            format!("tegemezi '{name}' halipo kwenye pata.toml"),
            1,
        ));
    }

    let root = Path::new(".");
    let path = root.join("pata.toml");
    let content = fs::read_to_string(&path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1))?;

    let mut out = Vec::new();
    let mut in_dep = false;

    for raw in content.lines() {
        let line = raw.to_string();
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_dep = trimmed == "[tegemezi]";
            out.push(line);
            continue;
        }

        if in_dep {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == name {
                    continue;
                }
            }
        }

        out.push(line);
    }

    let final_content = format!("{}\n", out.join("\n"));
    fs::write(&path, final_content)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", path.display()), 1))?;

    cfg.dependencies.remove(name);
    println!("tegemezi '{name}' imeondolewa");
    Ok(())
}

/// Convert a CLI-facing `Dependency` into the `pata-package` crate's manifest representation, so
/// resolution/locking can delegate to `pata_package::resolver::Resolver` and
/// `pata_package::lock::LockFile` instead of hand-rolling version resolution and checksums.
fn to_package_dependency(dep: &Dependency) -> pata_package::manifest::Dependency {
    match dep {
        Dependency::Version(v) => pata_package::manifest::Dependency::Version(v.clone()),
        Dependency::Path(p) => pata_package::manifest::Dependency::Table(pata_package::manifest::DependencyTable {
            version: "0.0.0".to_string(),
            path: Some(p.to_string_lossy().to_string()),
            git: None,
            branch: None,
        }),
        Dependency::Git { url, version } => pata_package::manifest::Dependency::Table(pata_package::manifest::DependencyTable {
            version: version.clone(),
            path: None,
            git: Some(url.clone()),
            branch: None,
        }),
    }
}

/// Read pata.lock when present and return locked dependency versions for deterministic builds.
/// Delegates parsing to `pata_package::lock::LockFile` (real TOML, per-dependency SHA-256
/// checksums) instead of the previous hand-rolled `[dependencies]` line parser, which could not
/// round-trip path dependencies (their `{ path = "..." }` table syntax was unparseable on read).
pub fn read_lockfile(root: &Path) -> Result<Option<BTreeMap<String, Dependency>>, CliError> {
    let path = root.join("pata.lock");
    if !path.is_file() {
        return Ok(None);
    }
    let lock = pata_package::LockFile::load(&path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1))?;
    let mut deps: BTreeMap<String, Dependency> = BTreeMap::new();
    for (name, locked) in &lock.dependencies {
        let dep = match &locked.path {
            Some(p) => Dependency::Path(PathBuf::from(p)),
            None if locked.source == "git" => Dependency::Git {
                // The lockfile only records the resolved exact version, not the original
                // constraint given at `pata ongeza --git` time — using it as an exact-match
                // constraint here is correct for `find_module_file`'s purposes (it only checks
                // *that* this is a Version-or-Git-shaped dependency to unlock the vendored-cache
                // lookup branch, never re-parses this string as a semver::VersionReq).
                url: String::new(),
                version: locked.version.clone(),
            },
            None => Dependency::Version(locked.version.clone()),
        };
        deps.insert(name.clone(), dep);
    }
    Ok(Some(deps))
}

/// Re-hash every vendored git/registry dependency under `.asili/packages/` and compare against
/// the checksum `pata.lock` recorded at fetch time — the actual security property a lockfile is
/// for: catching a `.asili/packages/<name>/` directory that was swapped or edited after `pata
/// ongeza` fetched it, which the checksum being merely *written* (and never re-checked) could not
/// detect before. Returns the mismatching dependency names, empty when nothing is locked yet or
/// everything still matches — never errors just because there's no lockfile (a fresh single-file
/// build has none).
pub fn verify_lockfile_integrity(root: &Path) -> Result<Vec<pata_package::IntegrityMismatch>, CliError> {
    let path = root.join("pata.lock");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let lock = pata_package::LockFile::load(&path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1))?;
    lock.verify_content_integrity(root)
        .map_err(|e| CliError::new(format!("imeshindwa kuthibitisha uadilifu wa tegemezi: {e}"), 1))
}

/// Resolve `cfg.dependencies` and write pata.lock via `pata_package`'s resolver/lock format.
pub fn write_lockfile(root: &Path, cfg: &ProjectConfig) -> Result<(), CliError> {
    let pkg_deps: BTreeMap<String, pata_package::manifest::Dependency> = cfg
        .dependencies
        .iter()
        .map(|(k, v)| (k.clone(), to_package_dependency(v)))
        .collect();
    let existing = pata_package::LockFile::load(root.join("pata.lock")).ok();
    let mut lock = pata_package::Resolver::resolve(root, &pkg_deps, existing.as_ref())
        .map_err(|e| CliError::new(format!("imeshindwa kutatua tegemezi: {e}"), 1))?;
    // Keep the existing timestamp when the resolved dependency set is unchanged, so re-running
    // `pata jenga`/`pata ongeza` without dependency changes doesn't churn pata.lock every build.
    if let Some(prev) = &existing {
        if prev.dependencies == lock.dependencies {
            lock.locked_at = prev.locked_at.clone();
        }
    }

    let path = root.join("pata.lock");
    lock.save(&path)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", path.display()), 1))
}

#[cfg(test)]
mod tests {
    use super::{write_lockfile, Dependency, ProjectConfig};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lockfile_is_deterministic() {
        let tmp = temp_dir();
        // Path dependencies (unlike bare-version ones) resolve with no registry/vendor lookup
        // at all — the right fixture for a test whose only concern is write_lockfile's output
        // determinism across repeated runs, not real constraint-solving behavior (covered by
        // pata-package's own resolver tests).
        let mut deps = BTreeMap::new();
        deps.insert("a".into(), Dependency::Path(PathBuf::from("../a")));
        deps.insert("b".into(), Dependency::Path(PathBuf::from("../b")));
        let cfg = ProjectConfig {
            name: "app".into(),
            version: "0.1.0".into(),
            asili_version: "1.1".into(),
            entrypoint: PathBuf::from("src/kuu.as"),
            dependencies: deps,
            target: None,
            eneo_kazi: None,
        };
        write_lockfile(&tmp, &cfg).expect("lock 1");
        let a = fs::read_to_string(tmp.join("pata.lock")).expect("read a");
        write_lockfile(&tmp, &cfg).expect("lock 2");
        let b = fs::read_to_string(tmp.join("pata.lock")).expect("read b");
        assert_eq!(a, b);
        let _ = fs::remove_dir_all(tmp);
    }

    fn temp_dir() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-lock-{stamp}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }
}

/// A workspace root: a `pata.toml` declaring `[eneo-kazi] wanachama = [...]` (relative paths to
/// member project directories, each with its own `pata.toml`). Replaces the earlier design of a
/// separate `Asili.toml`/`pata_package::Workspace` (English-keyed, its own `Manifest`/
/// `WorkspaceConfig` schema unrelated to `pata.toml`'s real Swahili one) — one manifest file and
/// one syntax for every project, workspace root or not.
#[derive(Debug, Clone)]
pub struct PataWorkspace {
    pub root: PathBuf,
    /// Member directory name -> resolved absolute path, sorted for stable iteration/display.
    pub members: BTreeMap<String, PathBuf>,
}

/// Walk upward from `start` looking for a `pata.toml` with a non-empty `[eneo-kazi]` table —
/// the workspace root marker. Unlike the old `Asili.toml` design, this reuses the same
/// `load_project_config`/real-TOML parse every other `pata.toml` read goes through, so a
/// malformed `[eneo-kazi]` table surfaces the same way any other manifest error would.
pub fn find_workspace_root(start: &Path) -> Option<PataWorkspace> {
    let mut dir = start;
    loop {
        if dir.join("pata.toml").is_file() {
            if let Ok(cfg) = load_project_config(dir) {
                if let Some(member_dirs) = cfg.eneo_kazi {
                    let members: BTreeMap<String, PathBuf> = member_dirs
                        .into_iter()
                        .filter(|m| dir.join(m).join("pata.toml").is_file())
                        .map(|m| (m.clone(), dir.join(&m)))
                        .collect();
                    if !members.is_empty() {
                        return Some(PataWorkspace { root: dir.to_path_buf(), members });
                    }
                }
            }
        }
        dir = dir.parent()?;
    }
}
