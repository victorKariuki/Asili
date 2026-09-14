use std::fs;
use std::path::PathBuf;
use clap::Parser;
use pata_lint::lint_source;

#[derive(Parser)]
#[command(name = "pata-lint")]
#[command(about = "Ukaguzi wa chanzo cha Asili", long_about = None)]
struct Args {
    /// Faili au saraka ya kukagua
    path: Option<PathBuf>,

    /// Ripoti makosa tu, si maonyo
    #[arg(long = "makosa-tu")]
    errors_only: bool,
}

fn main() {
    let args = Args::parse();
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    if path.is_file() {
        if let Err(e) = lint_file(&path, args.errors_only) {
            eprintln!("Kosa: {}", e);
        }
    } else if path.is_dir() {
        if let Err(e) = lint_directory(&path, args.errors_only) {
            eprintln!("Kosa: {}", e);
        }
    } else {
        eprintln!("Kosa: {} si faili wala saraka", path.display());
    }
}

fn lint_file(path: &PathBuf, errors_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    match lint_source(&source) {
        Ok(diags) => {
            if diags.is_empty() {
                println!("✓ {} - hakuna tatizo lililopatikana", path.display());
            } else {
                for diag in diags {
                    if errors_only && diag.stage == "ukaguzi" {
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
        Err(e) => Err(format!("imeshindwa kukagua {}: {}", path.display(), e).into()),
    }
}

fn lint_directory(dir: &PathBuf, errors_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut had_error = false;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // `.asi` files are interface *stubs* — bare signatures like
        // `kazi chapisha(ujumbe: Neno) -> Tupu` with no body, meant for
        // `InterfaceRegistry`'s own lenient line-based reader, not the full `.as` grammar (see
        // the matching NOTE in pata/lsp/src/lib.rs::is_interface_stub). Linting one with the
        // real parser always fails — a real `kazi ... -> T` with no `{ }` is a parse error —
        // and there's nothing a style/naming/complexity linter meaningfully checks on a bare
        // signature anyway.
        if path.is_file() && path.extension().map(|e| e == "as").unwrap_or(false) {
            // Don't let one unparseable file stop the rest of the tree from being linted —
            // report it and keep going, matching how a linter is expected to behave.
            if let Err(e) = lint_file(&path, errors_only) {
                eprintln!("{e}");
                had_error = true;
            }
        } else if path.is_dir() && !path.file_name().map(|n| n == "kilele").unwrap_or(false) {
            if lint_directory(&path, errors_only).is_err() {
                had_error = true;
            }
        }
    }

    if had_error {
        return Err("faili moja au zaidi imeshindwa kukaguliwa".into());
    }
    Ok(())
}
