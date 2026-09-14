use std::fs;
use std::path::PathBuf;
use clap::Parser;
use pata_lint::{config::LintConfig, lint_source_with_config};

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

    // `[lint.rules]` is read from the nearest ancestor `pata.toml`, walking up from `path` —
    // usually a subdirectory/file below the real project root, not the root itself. A project
    // with no `pata.toml` anywhere above `path`, or no `[lint.rules]` table, lints with every
    // rule at its default severity/threshold — `find_and_load` returns `LintConfig::new()` in
    // that case, not an error.
    let config = LintConfig::find_and_load(&path).unwrap_or_else(|e| {
        eprintln!("Onyo: imeshindwa kusoma usanidi wa pata.toml: {e}");
        LintConfig::new()
    });

    if path.is_file() {
        if let Err(e) = lint_file(&path, args.errors_only, &config) {
            eprintln!("Kosa: {}", e);
        }
    } else if path.is_dir() {
        if let Err(e) = lint_directory(&path, args.errors_only, &config) {
            eprintln!("Kosa: {}", e);
        }
    } else {
        eprintln!("Kosa: {} si faili wala saraka", path.display());
    }
}

fn lint_file(path: &PathBuf, errors_only: bool, config: &LintConfig) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    match lint_source_with_config(&source, config) {
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

fn lint_directory(dir: &PathBuf, errors_only: bool, config: &LintConfig) -> Result<(), Box<dyn std::error::Error>> {
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
            if let Err(e) = lint_file(&path, errors_only, config) {
                eprintln!("{e}");
                had_error = true;
            }
        } else if path.is_dir() && !path.file_name().map(|n| n == "kilele").unwrap_or(false) {
            if lint_directory(&path, errors_only, config).is_err() {
                had_error = true;
            }
        }
    }

    if had_error {
        return Err("faili moja au zaidi imeshindwa kukaguliwa".into());
    }
    Ok(())
}
