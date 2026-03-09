mod commands;
mod pipeline;

use commands::{dispatch, CliError};

fn main() {
    if let Err(err) = run() {
        eprintln!("kosa: {}", err.message);
        std::process::exit(err.exit_code);
    }
}

fn run() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    dispatch(&args)
}
