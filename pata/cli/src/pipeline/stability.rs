//! Type-stability checking: diff the current project's public API against the most recent
//! `v<semver>` git tag, if one exists. No tag → no check (not an error; a project with no
//! releases yet has nothing to be stable against). Flags: a public function present at the tag
//! but missing now; a function present at both with a different parameter count, a different
//! parameter type at the same position, or a different return type.
//!
//! Struct/trait signature changes are out of scope for this check — extending it to those is a
//! mechanical repeat of the same pattern once this lands, not a new design; tracked as a
//! follow-up, not silently dropped (see `docs/design/pata-implementation-spec.md` Section 8).

use crate::commands::CliError;
use asili_parser::Module;
use std::path::Path;

/// Check type stability against the most recent `v<semver>` git tag in the repo at `root`. Does
/// nothing (returns `Ok`) when `root` isn't a git repository, or has no tag matching `v<semver>`
/// — both are legitimate "nothing to check yet" states, not errors.
pub fn enforce_type_stability(root: &Path, current: &Module) -> Result<(), CliError> {
    let repo = match git2::Repository::open(root) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };

    let Some(tag_name) = most_recent_semver_tag(&repo) else {
        return Ok(());
    };

    let Some(baseline_src) = read_entrypoint_at_tag(&repo, &tag_name, root) else {
        // Tag exists but its entrypoint is unreadable at that revision (e.g. pata.toml or the
        // entrypoint file didn't exist yet at that point in history) — skip rather than fail
        // the build over a baseline this check can't actually establish.
        return Ok(());
    };

    let Ok(baseline_tokens) = asili_lexer::tokenize(&baseline_src) else { return Ok(()) };
    let Ok(baseline_module) = asili_parser::parse_tokens(&baseline_tokens) else { return Ok(()) };

    let mut breaking_changes = Vec::new();

    for old_fn in baseline_module.functions.iter().filter(|f| f.is_public) {
        let Some(new_fn) = current.functions.iter().find(|f| f.name == old_fn.name) else {
            breaking_changes.push(format!(
                "kazi ya umma '{}' iliyopo kwenye {} imeondolewa — mabadiliko yanayovunja API",
                old_fn.name, tag_name
            ));
            continue;
        };

        if new_fn.params.len() != old_fn.params.len() {
            breaking_changes.push(format!(
                "kazi ya umma '{}' idadi ya hoja imebadilika tangu {} ({} -> {})",
                old_fn.name, tag_name, old_fn.params.len(), new_fn.params.len()
            ));
            continue; // a length mismatch makes per-position comparison below meaningless
        }

        for (op, np) in old_fn.params.iter().zip(new_fn.params.iter()) {
            if op.ty.name != np.ty.name {
                breaking_changes.push(format!(
                    "kazi ya umma '{}' hoja '{}' aina imebadilika tangu {} ({} -> {})",
                    old_fn.name, np.name, tag_name, op.ty.name, np.ty.name
                ));
            }
        }

        if old_fn.return_type.name != new_fn.return_type.name {
            breaking_changes.push(format!(
                "kazi ya umma '{}' aina ya kurejesha imebadilika tangu {} ({} -> {})",
                old_fn.name, tag_name, old_fn.return_type.name, new_fn.return_type.name
            ));
        }
    }

    if !breaking_changes.is_empty() {
        return Err(CliError::new(breaking_changes.join("\n"), 1));
    }

    Ok(())
}

/// Highest `v<semver>` tag by real semver ordering (not lexical — `v2.0.0` must not sort after
/// `v10.0.0`), or `None` if no tag matches the pattern.
fn most_recent_semver_tag(repo: &git2::Repository) -> Option<String> {
    let tags = repo.tag_names(Some("v*")).ok()?;
    tags.iter()
        .flatten()
        .filter_map(|t| {
            let ver_str = t.strip_prefix('v')?;
            semver::Version::parse(ver_str).ok().map(|v| (v, t.to_string()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, tag)| tag)
}

/// Read the project's entrypoint source at `tag`'s git tree — reads `pata.toml`'s `[chanzo]
/// kuingia` path AT THE TAG too (not the working tree's current manifest), since the entrypoint
/// path itself could theoretically have changed since the tag.
fn read_entrypoint_at_tag(repo: &git2::Repository, tag: &str, _root: &Path) -> Option<String> {
    let obj = repo.revparse_single(tag).ok()?;
    let commit = obj.peel_to_commit().ok()?;
    let tree = commit.tree().ok()?;

    let pata_toml_entry = tree.get_path(Path::new("pata.toml")).ok()?;
    let pata_toml_blob = pata_toml_entry.to_object(repo).ok()?.peel_to_blob().ok()?;
    let pata_toml_content = std::str::from_utf8(pata_toml_blob.content()).ok()?;

    // Minimal inline extraction of [chanzo] kuingia — doesn't call load_project_config, which
    // reads from disk via std::fs, not from a git tree object; re-implementing its 5-line
    // [chanzo] section scan inline is simpler than refactoring it to take a string.
    let mut in_chanzo = false;
    let mut entry_rel = None;
    for line in pata_toml_content.lines() {
        let line = line.trim();
        if line == "[chanzo]" {
            in_chanzo = true;
            continue;
        }
        if line.starts_with('[') {
            in_chanzo = false;
        }
        if in_chanzo {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "kuingia" {
                    entry_rel = Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    let entry_rel = entry_rel.unwrap_or_else(|| "src/kuu.as".to_string());

    let entry_entry = tree.get_path(Path::new(&entry_rel)).ok()?;
    let entry_blob = entry_entry.to_object(repo).ok()?.peel_to_blob().ok()?;
    std::str::from_utf8(entry_blob.content()).ok().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git command should spawn");
        assert!(status.success(), "git {:?} failed in {}", args, dir.display());
    }

    fn temp_repo() -> std::path::PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-stability-test-{stamp}"));
        fs::create_dir_all(dir.join("src")).unwrap();
        dir
    }

    fn write_project(dir: &Path, kuu_src: &str) {
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        ).unwrap();
        fs::write(dir.join("src/kuu.as"), kuu_src).unwrap();
    }

    fn parse(src: &str) -> Module {
        let tokens = asili_lexer::tokenize(src).expect("tokenize");
        asili_parser::parse_tokens(&tokens).expect("parse")
    }

    #[test]
    fn no_check_when_not_a_git_repo() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f() -> Tupu { rejesha Tupu }\n");
        let current = parse("umma kazi f(a: Namba) -> Tupu { rejesha Tupu }\n");

        // Not a git repo at all -- must not error even though the (hypothetical) signature
        // changed, since there's no baseline to compare against.
        let result = enforce_type_stability(&dir, &current);
        assert!(result.is_ok());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_check_when_no_semver_tags() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f() -> Tupu { rejesha Tupu }\n");
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.com"]);
        run_git(&dir, &["config", "user.name", "T"]);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "initial"]);
        // No tag at all.

        let current = parse("umma kazi f(a: Namba) -> Tupu { rejesha Tupu }\n");
        let result = enforce_type_stability(&dir, &current);
        assert!(result.is_ok());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fails_on_removed_public_function() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f() -> Tupu { rejesha Tupu }\numma kazi g() -> Tupu { rejesha Tupu }\n");
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.com"]);
        run_git(&dir, &["config", "user.name", "T"]);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "initial"]);
        run_git(&dir, &["tag", "v1.0.0"]);

        // g() removed in the "current" module (in-memory, doesn't need a new commit).
        let current = parse("umma kazi f() -> Tupu { rejesha Tupu }\n");
        let err = enforce_type_stability(&dir, &current).expect_err("removed public fn should fail");
        assert!(err.message.contains("'g'"), "{}", err.message);
        assert!(err.message.contains("imeondolewa"), "{}", err.message);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fails_on_changed_arity() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f() -> Tupu { rejesha Tupu }\n");
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.com"]);
        run_git(&dir, &["config", "user.name", "T"]);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "initial"]);
        run_git(&dir, &["tag", "v1.0.0"]);

        let current = parse("umma kazi f(a: Namba) -> Tupu { rejesha Tupu }\n");
        let err = enforce_type_stability(&dir, &current).expect_err("arity change should fail");
        assert!(err.message.contains("idadi ya hoja"), "{}", err.message);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fails_on_changed_param_type() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f(a: Namba) -> Tupu { rejesha Tupu }\n");
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.com"]);
        run_git(&dir, &["config", "user.name", "T"]);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "initial"]);
        run_git(&dir, &["tag", "v1.0.0"]);

        let current = parse("umma kazi f(a: Neno) -> Tupu { rejesha Tupu }\n");
        let err = enforce_type_stability(&dir, &current).expect_err("param type change should fail");
        assert!(err.message.contains("aina imebadilika"), "{}", err.message);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fails_on_changed_return_type() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f() -> Namba { rejesha 1 }\n");
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.com"]);
        run_git(&dir, &["config", "user.name", "T"]);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "initial"]);
        run_git(&dir, &["tag", "v1.0.0"]);

        let current = parse("umma kazi f() -> Neno { rejesha \"x\" }\n");
        let err = enforce_type_stability(&dir, &current).expect_err("return type change should fail");
        assert!(err.message.contains("aina ya kurejesha"), "{}", err.message);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn passes_when_public_api_unchanged() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f(a: Namba) -> Neno { rejesha \"x\" }\n");
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.com"]);
        run_git(&dir, &["config", "user.name", "T"]);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "initial"]);
        run_git(&dir, &["tag", "v1.0.0"]);

        let current = parse("umma kazi f(a: Namba) -> Neno { rejesha \"x\" }\numma kazi mpya() -> Tupu { rejesha Tupu }\n");
        let result = enforce_type_stability(&dir, &current);
        assert!(result.is_ok(), "adding a new public function must not be flagged as breaking");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn picks_highest_semver_tag_not_lexically_last() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let dir = temp_repo();
        write_project(&dir, "umma kazi f() -> Tupu { rejesha Tupu }\n");
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.com"]);
        run_git(&dir, &["config", "user.name", "T"]);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "initial"]);
        run_git(&dir, &["tag", "v2.0.0"]);
        run_git(&dir, &["tag", "v10.0.0"]);

        let repo = git2::Repository::open(&dir).unwrap();
        let tag = most_recent_semver_tag(&repo).expect("a tag should be found");
        assert_eq!(tag, "v10.0.0", "v10.0.0 must sort after v2.0.0 by real semver ordering, not lexically");

        fs::remove_dir_all(&dir).ok();
    }
}
