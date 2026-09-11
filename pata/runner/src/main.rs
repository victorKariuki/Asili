//! Standalone runner: load .asb (or artifact from .build.manifest) and run kuu.
//! No compiler, parser, or project logic — minimal deployment footprint.

use asili_evaluator::{load_asb, load_asb_bytecode, parse_format, run_bytecode, run_main};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("matumizi: tenda <path.asb | path.build.manifest> [hoja za kuu...]");
        process::exit(2);
    }
    if args[1] == "--help" || args[1] == "-h" || args[1] == "--msaada" {
        println!("matumizi: tenda <path.asb | path.build.manifest> [hoja za kuu...]\n\nTenda kilele bila kujenga. Hoja za kuu zinapewa kwa kuu(hoja: Orodha<Neno>).");
        return;
    }
    let path = PathBuf::from(&args[1]);
    let program_args: Vec<String> = args[2..].to_vec();
    if !path.exists() {
        eprintln!("faili haipo: {}", path.display());
        process::exit(1);
    }
    let asb_path = if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".build.manifest"))
    {
        artifact_from_manifest(&path).unwrap_or_else(|e| {
            eprintln!("{e}");
            process::exit(1);
        })
    } else if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".asb"))
    {
        path
    } else {
        eprintln!("tenda inahitaji .asb au .build.manifest, si: {}", path.display());
        process::exit(2);
    };
    if !asb_path.exists() {
        eprintln!("kilele haipo: {}", asb_path.display());
        process::exit(1);
    }
    let bytes = fs::read(&asb_path).unwrap_or_else(|e| {
        eprintln!("imeshindwa kusoma {}: {e}", asb_path.display());
        process::exit(1);
    });
    let format = parse_format(&bytes).unwrap_or_else(|| "serialized".to_string());
    if format == "bytecode" {
        let program = load_asb_bytecode(&bytes).unwrap_or_else(|e| {
            eprintln!("kuipakia asb bytecode: {e}");
            process::exit(1);
        });
        if let Err(e) = run_bytecode(&program, program_args) {
            eprintln!("kuendesha kuu: {e}");
            process::exit(1);
        }
    } else {
        let module = load_asb(&bytes).unwrap_or_else(|e| {
            eprintln!("kuipakia asb: {e}");
            process::exit(1);
        });
        if let Err(e) = run_main(&module, program_args) {
            eprintln!("kuendesha kuu: {e}");
            process::exit(1);
        }
    }
}

fn artifact_from_manifest(manifest_path: &Path) -> Result<PathBuf, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|e| format!("imeshindwa kusoma manifest: {e}"))?;
    let artifact = content
        .lines()
        .find(|l| l.starts_with("kilele="))
        .and_then(|l| l.strip_prefix("kilele=").map(str::trim))
        .ok_or_else(|| "manifest haina mstari kilele=".to_string())?;
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(manifest_dir.join(artifact))
}
