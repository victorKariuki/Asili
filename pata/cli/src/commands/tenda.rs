//! Run from .asb artifact or .build.manifest (no recompile).

use super::{CliError, CliResult};
use asili_evaluator::{load_asb, load_asb_bytecode, parse_format, run_bytecode, run_main};
use std::fs;
use std::path::{Path, PathBuf};

const TENDA_USAGE: &str = r#"matumizi: pata tenda <path.asb | path.build.manifest> [hoja za kuu...]

Tenda kilele kilichojengwa bila kujenga tena.

  path.asb             Faili ya bytecode; tenda moja kwa moja.
  path.build.manifest  Soma kilele= kutoka manifest, kisha tenda .asb ile.

Hoja za kuu: zinapewa kwa kuu(hoja: Orodha<Neno>).

Mfano:
  pata tenda kilele/hello.asb
  pata tenda kilele/hello.build.manifest foo bar
"#;

/// Resolve artifact path from a .build.manifest file. Returns path to .asb (relative to manifest dir or absolute).
fn artifact_from_manifest(manifest_path: &Path) -> Result<PathBuf, CliError> {
    let content = fs::read_to_string(manifest_path).map_err(|e| {
        CliError::new(format!("imeshindwa kusoma manifest {}: {e}", manifest_path.display()), 1)
    })?;
    let artifact = content
        .lines()
        .find(|l| l.starts_with("kilele="))
        .and_then(|l| l.strip_prefix("kilele=").map(str::trim))
        .ok_or_else(|| {
            CliError::new(
                format!("manifest {} haina mstari kilele=", manifest_path.display()),
                1,
            )
        })?;
    let manifest_dir = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    Ok(manifest_dir.join(artifact))
}

pub fn run(args: &[String]) -> CliResult {
    if args
        .iter()
        .any(|a| a == "--msaada")
    {
        print!("{TENDA_USAGE}");
        return Ok(());
    }
    let (artifact_path, program_args) = match args.split_first() {
        Some((path, rest)) => (PathBuf::from(path), rest.to_vec()),
        None => {
            return Err(CliError::new(
                "tenda inahitaji path: .asb au .build.manifest".to_string(),
                2,
            ));
        }
    };
    if !artifact_path.exists() {
        return Err(CliError::new(
            format!("faili haipo: {}", artifact_path.display()),
            1,
        ));
    }
    let asb_path: PathBuf = if artifact_path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".build.manifest"))
    {
        artifact_from_manifest(&artifact_path)?
    } else if artifact_path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".asb"))
    {
        artifact_path.clone()
    } else {
        return Err(CliError::new(
            format!(
                "tenda inahitaji .asb au .build.manifest, si: {}",
                artifact_path.display()
            ),
            2,
        ));
    };
    if !asb_path.exists() {
        return Err(CliError::new(
            format!("artifact haipo: {} (kutoka manifest)", asb_path.display()),
            1,
        ));
    }
    let bytes = fs::read(&asb_path).map_err(|e| {
        CliError::new(
            format!("imeshindwa kusoma {}: {e}", asb_path.display()),
            1,
        )
    })?;
    let format = parse_format(&bytes).unwrap_or_else(|| "serialized".to_string());
    if format == "bytecode" {
        let program = load_asb_bytecode(&bytes).map_err(|e| {
            CliError::new(format!("kuipakia asb bytecode: {e}"), 1)
        })?;
        run_bytecode(&program, program_args).map_err(|e| {
            CliError::new(format!("kuendesha kuu: {e}"), 1)
        })?;
    } else {
        let module = load_asb(&bytes).map_err(|e| {
            CliError::new(format!("kuipakia asb: {e}"), 1)
        })?;
        run_main(&module, program_args).map_err(|e| {
            CliError::new(format!("kuendesha kuu: {e}"), 1)
        })?;
    }
    Ok(())
}
