//! Standalone runner: load .asb (or artifact from .build.manifest) and run kuu.
//! No compiler, parser, or project logic — minimal deployment footprint.

use asili_evaluator::run_artifact;
use std::env;
use std::path::Path;
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
    if let Err(e) = run_artifact(Path::new(&args[1]), args[2..].to_vec()) {
        eprintln!("{e}");
        process::exit(e.exit_code());
    }
}
