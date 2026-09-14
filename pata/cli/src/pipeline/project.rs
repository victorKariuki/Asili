use crate::commands::CliError;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum Dependency {
    Version(String),
    Path(PathBuf),
}

impl From<&str> for Dependency {
    fn from(s: &str) -> Self {
        Dependency::Version(s.to_string())
    }
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dependency::Version(v) => write!(f, "\"{}\"", v),
            Dependency::Path(p) => write!(f, "{{ path = \"{}\" }}", p.display()),
        }
    }
}

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
}

pub fn load_project_config(root: &Path) -> Result<ProjectConfig, CliError> {
    let path = root.join("pata.toml");
    let content = fs::read_to_string(&path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1))?;

    let mut section = String::new();
    let mut name = String::new();
    let mut version = String::new();
    let mut asili_version = String::new();
    let mut entry = PathBuf::from("src/kuu.as");
    let mut deps: BTreeMap<String, Dependency> = BTreeMap::new();
    let mut target: Option<String> = None;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(&['[', ']'][..]).to_string();
            continue;
        }

        if section == "tegemezi" {
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().to_string();
                let val = v.trim();
                if val.starts_with('{') && val.ends_with('}') {
                    let inner = val.trim_matches(&['{', '}'][..]);
                    if let Some((pk, pv)) = inner.split_once('=') {
                        if pk.trim() == "path" {
                            deps.insert(key, Dependency::Path(PathBuf::from(pv.trim().trim_matches('"'))));
                        }
                    }
                } else {
                    deps.insert(key, Dependency::Version(val.trim_matches('"').to_string()));
                }
            }
            continue;
        }

        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let value = v.trim().trim_matches('"').to_string();

        match section.as_str() {
            "jumla" => match key {
                "jina" => name = value,
                "toleo" => version = value,
                "asili" => asili_version = value,
                _ => {}
            },
            "chanzo" if key == "kuingia" => {
                entry = PathBuf::from(value);
            }
            "jenga" if key == "lengo" => {
                target = Some(value);
            }
            _ => {}
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
                out.push(format!("{dep} = \"{version}\""));
                inserted = true;
            }
            in_dep = trimmed == "[tegemezi]";
            out.push(line);
            continue;
        }

        if in_dep {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == dep {
                    out.push(format!("{dep} = \"{version}\""));
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
        out.push(format!("{dep} = \"{version}\""));
    } else if !inserted {
        out.push(format!("{dep} = \"{version}\""));
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
            None => Dependency::Version(locked.version.clone()),
        };
        deps.insert(name.clone(), dep);
    }
    Ok(Some(deps))
}

/// Resolve `cfg.dependencies` and write pata.lock via `pata_package`'s resolver/lock format.
pub fn write_lockfile(root: &Path, cfg: &ProjectConfig) -> Result<(), CliError> {
    let pkg_deps: BTreeMap<String, pata_package::manifest::Dependency> = cfg
        .dependencies
        .iter()
        .map(|(k, v)| (k.clone(), to_package_dependency(v)))
        .collect();
    let existing = pata_package::LockFile::load(root.join("pata.lock")).ok();
    let mut lock = pata_package::Resolver::resolve(&pkg_deps, existing.as_ref())
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
    use super::{write_lockfile, ProjectConfig};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lockfile_is_deterministic() {
        let tmp = temp_dir();
        let mut deps = BTreeMap::new();
        deps.insert("a".into(), "^1.0".into());
        deps.insert("b".into(), "~2.0".into());
        let cfg = ProjectConfig {
            name: "app".into(),
            version: "0.1.0".into(),
            asili_version: "1.1".into(),
            entrypoint: PathBuf::from("src/kuu.as"),
            dependencies: deps,
            target: None,
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
