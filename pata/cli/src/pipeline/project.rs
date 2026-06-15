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
    pub version: String,
    pub asili_version: String,
    pub entrypoint: PathBuf,
    pub dependencies: BTreeMap<String, Dependency>,
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

/// Read pata.lock when present and return locked dependency versions for deterministic builds.
pub fn read_lockfile(root: &Path) -> Result<Option<BTreeMap<String, Dependency>>, CliError> {
    let path = root.join("pata.lock");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    let mut deps: BTreeMap<String, Dependency> = BTreeMap::new();
    let mut in_deps = false;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if in_deps {
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().trim_matches('"').to_string();
                let value = v.trim().trim_matches('"').to_string();
                deps.insert(key, Dependency::Version(value));
            }
        }
    }
    Ok(Some(deps))
}

pub fn write_lockfile(root: &Path, cfg: &ProjectConfig) -> Result<(), CliError> {
    let mut checksum_src = format!("{}|{}|{}", cfg.name, cfg.version, cfg.asili_version);
    for (k, v) in &cfg.dependencies {
        checksum_src.push('|');
        checksum_src.push_str(k);
        checksum_src.push('|');
        match v {
            Dependency::Version(ver) => checksum_src.push_str(ver),
            Dependency::Path(path) => checksum_src.push_str(&path.to_string_lossy()),
        }
    }
    let checksum = simple_hash(&checksum_src);

    let mut content = String::new();
    content.push_str("format = \"pata-lock-v1\"\n");
    content.push_str(&format!("project = \"{}\"\n", cfg.name));
    content.push_str(&format!("asili = \"{}\"\n", cfg.asili_version));
    content.push_str(&format!("checksum = \"{checksum:016x}\"\n\n"));
    content.push_str("[dependencies]\n");
    for (k, v) in &cfg.dependencies {
        content.push_str(&format!("{k} = {v}\n"));
    }

    let path = root.join("pata.lock");
    fs::write(&path, content)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", path.display()), 1))
}

// HACK: simple_hash is a FNV-1a variant used for the pata.lock checksum. It is not
// cryptographically secure — a malicious pata.toml could be crafted to produce a collision.
// For lock file integrity use a proper hash (SHA-256 via the `sha2` crate) to detect tampering.
fn simple_hash(s: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
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
