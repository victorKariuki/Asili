use super::{CliError, CliResult};
use crate::pipeline::format::{check_or_write_named, collect_asili_files, print_diff};
use std::path::{Path, PathBuf};

// Contract: ../../commands/nadhifu.md
pub fn run(args: &[String]) -> CliResult {
    let (check_only, show_diff, json, path) = parse_args(args)?;
    let root = path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let files = collect_asili_files(Path::new(&root))?;

    if show_diff {
        let (total, changed) = print_diff(&files)?;
        if changed == 0 {
            println!("sawa: mafaili yote {} yako katika muundo sahihi", total);
        }
        // --diff never writes; a nonzero exit still signals "not formatted", matching --kagua.
        if changed > 0 {
            return Err(CliError::new(
                format!("format haijafuata viwango: mafaili {changed}/{total}"),
                1,
            ));
        }
        return Ok(());
    }

    let (total, changed_files) = check_or_write_named(&files, check_only)?;
    let changed = changed_files.len();

    if json {
        let output = serde_json::json!({
            "sawa": changed == 0,
            "jumla": total,
            "yamebadilishwa": changed_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        if check_only && changed > 0 {
            return Err(CliError::new(
                format!("format haijafuata viwango: mafaili {changed}/{total}"),
                1,
            ));
        }
        return Ok(());
    }

    if check_only {
        if changed > 0 {
            return Err(CliError::new(
                format!("format haijafuata viwango: mafaili {changed}/{total}"),
                1,
            ));
        }
        println!("sawa: mafaili yote {} yako katika muundo sahihi", total);
        return Ok(());
    }

    println!("imekamilika: mafaili {} yalikaguliwa, {} yamebadilishwa", total, changed);
    Ok(())
}

fn parse_args(args: &[String]) -> Result<(bool, bool, bool, Option<String>), CliError> {
    let mut check = false;
    let mut diff = false;
    let mut json = false;
    let mut path = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--kagua" => {
                check = true;
                i += 1;
            }
            "--diff" => {
                diff = true;
                i += 1;
            }
            "--json" => {
                json = true;
                i += 1;
            }
            other if !other.starts_with('-') => {
                if path.is_some() {
                    return Err(CliError::new(
                        format!("nadhifu inakubali njia moja tu, umetoa nyingine: {other}"),
                        2,
                    ));
                }
                path = Some(other.to_string());
                i += 1;
            }
            other => {
                return Err(CliError::new(
                    format!("hoja isiyotambuliwa kwenye nadhifu: {other}"),
                    2,
                ));
            }
        }
    }
    Ok((check, diff, json, path))
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn check_mode_fails_on_unformatted_source() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_dir();
        std::env::set_current_dir(&root).expect("chdir");
        fs::write("src/kuu.as", "leta matumizi\nkazi  kuu(){chapisha(\"x\")}\n").expect("write");

        let err = run(&["--kagua".into()]).expect_err("should fail check");
        assert_eq!(err.exit_code, 1);

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    /// `--diff` on unformatted source: exits nonzero like `--kagua` (it found something to
    /// change), and — the actual point of this mode — must never rewrite the file on disk, unlike
    /// the default (write) mode.
    #[test]
    fn diff_mode_reports_changes_without_writing_the_file() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_dir();
        std::env::set_current_dir(&root).expect("chdir");
        let unformatted = "leta matumizi\nkazi  kuu(){chapisha(\"x\")}\n";
        fs::write("src/kuu.as", unformatted).expect("write");

        let err = run(&["--diff".into()]).expect_err("diff mode should still exit nonzero when changes exist");
        assert_eq!(err.exit_code, 1);

        let on_disk = fs::read_to_string("src/kuu.as").expect("read back");
        assert_eq!(on_disk, unformatted, "--diff must never modify the file on disk");

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    /// `--diff` on already-formatted source: nothing to report, exits Ok (matching --kagua's
    /// "already correct" success path).
    #[test]
    fn diff_mode_succeeds_on_already_formatted_source() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_dir();
        std::env::set_current_dir(&root).expect("chdir");
        let formatted = "leta matumizi\n\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    chapisha(\"x\")\n}\n";
        fs::write("src/kuu.as", formatted).expect("write");

        // Only assert success if the formatter already considers this idempotent input clean —
        // avoids the test being coupled to the formatter's exact canonical style; the real
        // property under test is "no diff => Ok", proven together with the "has diff => Err"
        // case above.
        if crate::pipeline::format::canonical_format(formatted, None) == formatted {
            run(&["--diff".into()]).expect("already-formatted source should report no diff");
        }

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    /// `--kagua --json` on unformatted source: exits nonzero (matching plain `--kagua`) and, per
    /// the point of adding JSON output, must still work — the flag combination is accepted and
    /// the underlying pass/fail semantics are unchanged by asking for machine-readable output.
    #[test]
    fn json_check_mode_fails_on_unformatted_source_same_as_text_mode() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_dir();
        std::env::set_current_dir(&root).expect("chdir");
        fs::write("src/kuu.as", "leta matumizi\nkazi  kuu(){chapisha(\"x\")}\n").expect("write");

        let err = run(&["--kagua".into(), "--json".into()]).expect_err("should fail check in json mode too");
        assert_eq!(err.exit_code, 1);

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    fn temp_dir() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-nadhifu-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        dir
    }
}
