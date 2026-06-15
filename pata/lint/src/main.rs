use std::fs;
use std::path::PathBuf;
use clap::Parser;
use pata_lint::lint_source;

#[derive(Parser)]
#[command(name = "pata-lint")]
#[command(about = "Linter for Asili source code", long_about = None)]
struct Args {
    /// File or directory to lint
    path: Option<PathBuf>,

    /// Only report errors, not warnings
    #[arg(long)]
    errors_only: bool,
}

fn main() {
    let args = Args::parse();
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    if path.is_file() {
        if let Err(e) = lint_file(&path, args.errors_only) {
            eprintln!("Error: {}", e);
        }
    } else if path.is_dir() {
        if let Err(e) = lint_directory(&path, args.errors_only) {
            eprintln!("Error: {}", e);
        }
    } else {
        eprintln!("Error: {} is not a file or directory", path.display());
    }
}

fn lint_file(path: &PathBuf, errors_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    match lint_source(&source) {
        Ok(diags) => {
            if diags.is_empty() {
                println!("✓ {} - No issues found", path.display());
            } else {
                for diag in diags {
                    if errors_only && diag.stage == "lint" {
                        continue;
                    }
                    let location = match &diag.span {
                        Some(span) => format!("{}:{}", span.line, span.column),
                        None => "1:1".to_string(),
                    };
                    println!(
                        "{}:{} ({}): {}",
                        path.display(),
                        location,
                        diag.code,
                        diag.message
                    );
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to lint {}: {}", path.display(), e).into()),
    }
}

fn lint_directory(dir: &PathBuf, errors_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && (path.extension().map(|e| e == "as" || e == "asi").unwrap_or(false)) {
            lint_file(&path, errors_only)?;
        } else if path.is_dir() && !path.file_name().map(|n| n == "target").unwrap_or(false) {
            lint_directory(&path, errors_only)?;
        }
    }

    Ok(())
}
