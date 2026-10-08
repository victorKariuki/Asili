//! `pata fuatilia <faili>`: show a binary trace (`binari:<faili>`) as the readable tree.

use super::{CliError, CliResult};

const FUATILIA_USAGE: &str = r#"matumizi: pata fuatilia <faili>

Soma faili ya ufuatiliaji wa binari (fremu za baiti 4 zilizoandikwa na
--fuatilia=binari:<faili> au ASILI_FUATILIA=binari:<faili>) na kuionyesha kama mti.
"#;

pub fn run(args: &[String]) -> CliResult {
    if args.iter().any(|a| a == "--msaada") {
        print!("{FUATILIA_USAGE}");
        return Ok(());
    }
    let [path] = args else {
        return Err(CliError::new(
            "fuatilia inahitaji faili moja ya ufuatiliaji",
            2,
        ));
    };
    let bytes = std::fs::read(path)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {path}: {e}"), 1))?;
    print!("{}", asili_trace::decode(&bytes));
    Ok(())
}
