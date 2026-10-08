//! Standalone runner: load .asb (or artifact from .build.manifest) and run kuu.
//! No compiler, parser, or project logic — minimal deployment footprint.

use asili_evaluator::run_artifact;
use std::env;
use std::path::Path;
use std::process;

// Small blocks from per-thread free lists: musl's allocator locks on every call.
#[global_allocator]
static ALLOC: asili_evaluator::alloc::AsiliAlloc = asili_evaluator::alloc::AsiliAlloc;

fn main() {
    // Our memcpy/memmove/memset/memcmp replace musl's in the static build.
    asili_mem::linked();
    // `ASILI_FUATILIA=mti|json|binari:<faili>` traces the run (standalone executables too).
    if let Err(e) = asili_trace::install_from_env() {
        eprintln!("{e}");
        process::exit(2);
    }
    let code = run();
    asili_trace::finish();
    if code != 0 {
        process::exit(code);
    }
}

fn run() -> i32 {
    // A standalone executable (`pata jenga` appended the program to this runner): every
    // argument is the program's own.
    if let Some(bundle) = asili_evaluator::bundle::embedded() {
        if let Err(e) = asili_evaluator::run_bundle(&bundle, env::args().skip(1).collect()) {
            eprintln!("{e}");
            return e.exit_code();
        }
        return 0;
    }
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("matumizi: tenda <path.asb | path.build.manifest> [hoja za kuu...]");
        return 2;
    }
    if args[1] == "--help" || args[1] == "-h" || args[1] == "--msaada" {
        println!("matumizi: tenda <path.asb | path.build.manifest> [hoja za kuu...]\n\nTenda kilele bila kujenga. Hoja za kuu zinapewa kwa kuu(hoja: Orodha<Neno>).\nASILI_FUATILIA=mti|json|binari:<faili> hufuatilia utekelezaji.");
        return 0;
    }
    if let Err(e) = run_artifact(Path::new(&args[1]), args[2..].to_vec()) {
        eprintln!("{e}");
        return e.exit_code();
    }
    0
}
