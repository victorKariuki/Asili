mod commands;
mod pipeline;
#[cfg(test)]
mod test_support;

use commands::{dispatch, CliError};

fn main() {
    // `ASILI_FUATILIA=mti|json|binari:<faili>` traces whatever the command builds or runs.
    let result = asili_trace::install_from_env()
        .map_err(|e| CliError::new(e, 2))
        .and_then(|_| run());
    asili_trace::finish();
    if let Err(err) = result {
        eprintln!("kosa: {}", err.message);
        std::process::exit(err.exit_code);
    }
}

fn run() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    dispatch(&args)
}
