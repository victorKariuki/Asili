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
