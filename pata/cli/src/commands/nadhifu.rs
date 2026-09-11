use super::{CliError, CliResult};
use crate::pipeline::format::{check_or_write, collect_asili_files};
use std::path::{Path, PathBuf};

// Contract: ../../commands/nadhifu.md
pub fn run(args: &[String]) -> CliResult {
    let (check_only, path) = parse_args(args)?;
    let root = path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let files = collect_asili_files(Path::new(&root))?;
    let (total, changed) = check_or_write(&files, check_only)?;

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

fn parse_args(args: &[String]) -> Result<(bool, Option<String>), CliError> {
    let mut check = false;
    let mut path = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--kagua" => {
                check = true;
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
    Ok((check, path))
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
