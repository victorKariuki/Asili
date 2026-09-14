use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

use pata_fmt::config::FormatterConfig;
use pata_fmt::format::canonical_format_with_indent;
use pata_fmt::walk::collect_asili_files;

#[derive(Parser)]
#[command(name = "pata fmt")]
#[command(about = "Nadhifisha faili za chanzo za Asili", long_about = None)]
struct Cli {
    /// Faili au saraka ya kunadhifisha
    #[arg(value_name = "NJIA")]
    path: Option<PathBuf>,

    /// Kagua mfumo bila kubadilisha faili
    #[arg(short = 'k', long = "kagua")]
    check: bool,

    /// Onyesha tofauti za faili zisizofuata mfumo
    #[arg(short = 't', long = "tofauti")]
    diff: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let path = cli.path.unwrap_or_else(|| PathBuf::from("."));
    let files = if path.is_dir() {
        collect_asili_files(&path)?
    } else {
        vec![path.clone()]
    };

    if files.is_empty() {
        println!("hapana faili ya .as au .asi iliyopatikana");
        return Ok(());
    }

    // `[fmt]` is read from the nearest ancestor `pata.toml`, walking up from the target path —
    // usually a subdirectory/file below the real project root. No `pata.toml` anywhere above it
    // formats with the default 100-char width / 4-space indent (line_width isn't applied yet —
    // see `canonical_format_with_indent`'s doc comment).
    let config = FormatterConfig::find_and_load(&path).unwrap_or_else(|e| {
        eprintln!("Onyo: imeshindwa kusoma usanidi wa pata.toml: {e}");
        FormatterConfig::default()
    });

    let (total, changed) = format_files(&files, cli.check, cli.diff, &config)?;

    if cli.check {
        if changed > 0 {
            println!("faili {} zinahitaji kuboreswa kwa mfumo", changed);
            std::process::exit(1);
        } else {
            println!("faili {} zina mfumo sahihi", total);
        }
    } else {
        println!("kuboreswa {} kwa {} faili", changed, total);
    }

    Ok(())
}

fn format_files(files: &[PathBuf], check_only: bool, show_diff: bool, config: &FormatterConfig) -> Result<(usize, usize)> {
    let mut changed = 0;
    let indent_unit = config.indent_unit();
    for file in files {
        let original = fs::read_to_string(file)
            .with_context(|| format!("imeshindwa kusoma {}", file.display()))?;
        let formatted = canonical_format_with_indent(&original, &indent_unit);

        if formatted != original {
            changed += 1;
            if show_diff {
                print_diff(&original, &formatted, file);
            }
            if !check_only {
                fs::write(file, &formatted)
                    .with_context(|| format!("imeshindwa kuandika {}", file.display()))?;
            }
        }
    }
    Ok((files.len(), changed))
}

fn print_diff(original: &str, formatted: &str, path: &Path) {
    println!("\n--- {}", path.display());
    println!("+++ {} (nadhifu)", path.display());
    for (i, (orig_line, fmt_line)) in original.lines().zip(formatted.lines()).enumerate() {
        if orig_line != fmt_line {
            println!("@@ mstari {} @@", i + 1);
            println!("- {}", orig_line);
            println!("+ {}", fmt_line);
        }
    }
}
