//! Run from .asb artifact or .build.manifest (no recompile).

use super::{CliError, CliResult};
use asili_evaluator::run_artifact;
use std::path::Path;

const TENDA_USAGE: &str = r#"matumizi: pata tenda [--fuatilia[=namna]] <path.asb | path.build.manifest> [hoja za kuu...]

Tenda kilele kilichojengwa bila kujenga tena.

  path.asb             Faili ya bytecode; tenda moja kwa moja.
  path.build.manifest  Soma kilele= kutoka manifest, kisha tenda .asb ile.

Hoja za kuu: zinapewa kwa kuu(hoja: Orodha<Neno>).

  --fuatilia[=namna]   Fuatilia utekelezaji: mti (chaguo-msingi), json, json:<faili>
                       au binari:<faili> (pia ASILI_FUATILIA).

Mfano:
  pata tenda kilele/hello.asb
  pata tenda kilele/hello.build.manifest foo bar
"#;

pub fn run(args: &[String]) -> CliResult {
    if args.iter().any(|a| a == "--msaada") {
        print!("{TENDA_USAGE}");
        return Ok(());
    }
    // Options come before the artifact; everything after it belongs to the program.
    let mut args = args;
    while let Some(traced) = args.first().and_then(|a| super::trace_flag(a)) {
        traced?;
        args = &args[1..];
    }
    let Some((path, program_args)) = args.split_first() else {
        return Err(CliError::new(
            "tenda inahitaji path: .asb au .build.manifest".to_string(),
            2,
        ));
    };
    run_artifact(Path::new(path), program_args.to_vec())
        .map_err(|e| CliError::new(e.to_string(), e.exit_code()))?;
    Ok(())
}
