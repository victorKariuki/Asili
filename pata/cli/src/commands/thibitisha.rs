use super::{CliError, CliResult};
use crate::pipeline::compile::compile_project;
use crate::pipeline::format::{check_or_write, collect_asili_files};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, Module};
use std::fs;
use std::path::Path;

// Contract: ../../commands/thibitisha.md
//
// thibitisha checks: project compiles (which also runs SEM105 trait-completeness for any impl
// that names a trait), public-item doc coverage, formatting compliance, project-wide trait
// completeness (every sifa reachable from an import has at least one impl somewhere in the
// project — SEM105 alone only catches an impl that names a trait and gets it wrong, not a trait
// that's never implemented at all), FFI-safety of #[kiunganishi]-tagged signatures, and (opt-in
// via --kiwango-cha-jaribio) test coverage.
//
// Not implemented: type-stability (breaking public-signature changes between versions) — see
// `pata semver` (planned) once a git-tag-based baseline exists; full ABI compatibility against a
// C-signature contract, since #[kiunganishi]/kiungo has no such contract yet (FFI is a
// documented Phase IV stub, core/evaluator/src/builtins/kiungo.rs). What thibitisha checks today
// for #[kiunganishi] (FFI-safe types) is the real, checkable prerequisite for that future check,
// not a placeholder.
pub fn run(args: &[String]) -> CliResult {
    let threshold = parse_args(args)?;

    let output = compile_project(Path::new("."), None)?;
    enforce_docs(Path::new("."))?;
    enforce_trait_completeness(&output.module)?;
    enforce_ffi_signatures(&output.module)?;

    let files = collect_asili_files(Path::new("."))?;
    let (_, changed) = check_or_write(&files, true)?;
    if changed > 0 {
        return Err(CliError::new(
            "mafaili hayajafuata muundo sahihi: tumia `pata nadhifu` kwanza",
            1,
        ));
    }

    if let Some(threshold) = threshold {
        enforce_test_coverage(&output.module, threshold)?;
    }

    println!("thibitisha: sawa");
    Ok(())
}

/// Every `sifa` reachable from the project (declared locally, or by a `leta`-ed dependency
/// module/.asi stub — both land in `module.traits` after merge_for_eval) must have at least one
/// `shughuli ya X kwa/​: Trait` impl somewhere in the project. `check_trait_completeness`
/// (SEM105, run during the compile step above) only checks an impl that already names a trait
/// against that trait's signatures — a trait with zero impls anywhere never gets flagged there,
/// since SEM105 iterates `module.impls`, not `module.traits`. Skips built-in seeded traits
/// (Inasomeka/Inandikika from `standard_traits()`, `line == 0`, same convention as
/// `enforce_docs` below) — those are always present regardless of project content and satisfied
/// by fiat for builtin types (Faili/Mkondo), not something a project could implement itself.
fn enforce_trait_completeness(module: &Module) -> CliResult {
    for t in &module.traits {
        if t.line == 0 {
            continue;
        }
        let implemented = module
            .impls
            .iter()
            .any(|i| i.trait_name.as_deref() == Some(t.name.as_str()));
        if !implemented {
            return Err(CliError::new(
                format!("sifa '{}' haina utekelezaji wowote kwenye mradi huu", t.name),
                1,
            ));
        }
    }
    Ok(())
}

/// FFI-safe primitive types for an `#[kiunganishi]`-tagged function's parameters/return type.
/// There is no C-signature declaration syntax yet (kiungo/FFI is a Phase IV stub — see
/// core/evaluator/src/builtins/kiungo.rs), so a real ABI-compatibility check (declared C sig vs.
/// actual signature) has nothing to compare against. This is the real, checkable prerequisite:
/// reject types that could never cross a C boundary safely regardless of what the eventual
/// C-signature contract looks like (heap-owning/GC'd/generic-container types).
fn is_ffi_safe(ty_name: &str) -> bool {
    // TypeExpr::name is built from raw lexemes including any generic brackets (e.g.
    // "Orodha < Namba >" for `Orodha<Namba>`, not "Orodha") — take the base name before any
    // `<`/whitespace so a bare-name check works regardless of a generic argument list.
    let base = ty_name.split(['<', ' ']).next().unwrap_or(ty_name);
    matches!(
        base,
        "Namba" | "Ukweli" | "Herufi" | "Tupu" | "Anuani"
            | "Biti8" | "Biti16" | "Biti32" | "Biti64"
            | "uBiti8" | "uBiti16" | "uBiti32" | "uBiti64"
    )
}

fn enforce_ffi_signatures(module: &Module) -> CliResult {
    for f in &module.functions {
        if !f.attrs.iter().any(|a| a.name == "kiunganishi") {
            continue;
        }
        for p in &f.params {
            if !is_ffi_safe(&p.ty.name) {
                return Err(CliError::new(
                    format!(
                        "kazi ya kiunganishi '{}' hoja '{}' ina aina isiyo salama kwa ABI ya C: {}",
                        f.name, p.name, p.ty.name
                    ),
                    1,
                ));
            }
        }
        if f.return_type.name != "Tupu" && !is_ffi_safe(&f.return_type.name) {
            return Err(CliError::new(
                format!(
                    "kazi ya kiunganishi '{}' aina ya kurejesha si salama kwa ABI ya C: {}",
                    f.name, f.return_type.name
                ),
                1,
            ));
        }
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Option<f64>, CliError> {
    let mut threshold = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--kiwango-cha-jaribio" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::new("--kiwango-cha-jaribio inahitaji thamani (0-100)", 2));
                };
                let parsed: f64 = v.parse().map_err(|_| {
                    CliError::new(format!("--kiwango-cha-jaribio inahitaji namba (0-100), si: {v}"), 2)
                })?;
                if !(0.0..=100.0).contains(&parsed) {
                    return Err(CliError::new(
                        format!("--kiwango-cha-jaribio inahitaji thamani kati ya 0 na 100, si: {parsed}"),
                        2,
                    ));
                }
                threshold = Some(parsed);
                i += 2;
            }
            other => {
                return Err(CliError::new(
                    format!("hoja isiyotambuliwa kwenye thibitisha: {other}"),
                    2,
                ));
            }
        }
    }
    Ok(threshold)
}

/// Ratio of public `kazi` with a corresponding `#[jaribio]` test to total public `kazi`,
/// checked against `threshold` percent. A public function counts as "covered" if a test
/// function with a matching name convention (`jaribio_<name>` or simply any #[jaribio]
/// function, since Asili has no call-graph/coverage instrumentation) exists — kept
/// deliberately simple: presence of at least `threshold`% as many #[jaribio] functions as
/// public kazi, not per-function attribution.
fn enforce_test_coverage(module: &Module, threshold: f64) -> CliResult {
    let public_count = module.functions.iter().filter(|f| f.is_public).count();
    if public_count == 0 {
        return Ok(());
    }
    let test_count = module.functions.iter().filter(|f| f.is_test).count();
    let ratio = (test_count as f64 / public_count as f64) * 100.0;
    if ratio < threshold {
        return Err(CliError::new(
            format!(
                "kiwango cha majaribio {:.1}% ni chini ya kiwango kinachohitajika {:.1}% (majaribio {test_count} kwa kazi za umma {public_count})",
                ratio, threshold
            ),
            1,
        ));
    }
    Ok(())
}

fn has_doc_before(lines: &[&str], line_1based: usize) -> bool {
    let mut i = line_1based.saturating_sub(2) as i32;
    while i >= 0 {
        let ln = lines[i as usize].trim();
        if ln.is_empty() {
            i -= 1;
            continue;
        }
        return ln.starts_with("///");
    }
    false
}

fn enforce_docs(root: &Path) -> CliResult {
    let files = collect_asili_files(root)?;
    for file in files {
        let src = fs::read_to_string(&file)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", file.display()), 1))?;
        let lines: Vec<&str> = src.lines().collect();
        let toks = match tokenize(&src) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let module = match parse_tokens(&toks) {
            Ok(m) => m,
            Err(_) => {
                for i in 0..lines.len() {
                    let ln = lines[i].trim();
                    if (ln.starts_with("umma kazi") || ln.starts_with("umma umbo")) && (i == 0 || !lines[i - 1].trim().starts_with("///")) {
                        return Err(CliError::new(
                            format!(
                                "nyaraka zimekosekana kwa item ya umma kwenye {}:{}",
                                file.display(),
                                i + 1
                            ),
                            1,
                        ));
                    }
                }
                continue;
            }
        };
        for f in &module.functions {
            if f.is_public && !has_doc_before(&lines, f.line) {
                return Err(CliError::new(
                    format!(
                        "nyaraka zimekosekana kwa kazi ya umma '{}' kwenye {}:{}",
                        f.name,
                        file.display(),
                        f.line
                    ),
                    1,
                ));
            }
        }
        for s in &module.structs {
            if s.is_public && !has_doc_before(&lines, s.line) {
                return Err(CliError::new(
                    format!(
                        "nyaraka zimekosekana kwa umbo la umma '{}' kwenye {}:{}",
                        s.name,
                        file.display(),
                        s.line
                    ),
                    1,
                ));
            }
        }
        for t in &module.traits {
            // Skip language-seeded traits (Inasomeka, Inandikika — see standard_traits() in
            // core/parser/src/parse.rs): they're hardcoded Rust literals with no real source
            // location (line 0), not something any project file could ever add a doc comment
            // above, and they appear in every module's `traits` list regardless of file content.
            if t.line == 0 {
                continue;
            }
            if t.is_public && !has_doc_before(&lines, t.line) {
                return Err(CliError::new(
                    format!(
                        "nyaraka zimekosekana kwa sifa ya umma '{}' kwenye {}:{}",
                        t.name,
                        file.display(),
                        t.line
                    ),
                    1,
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fails_when_public_item_has_no_docs() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&[]).expect_err("thibitisha should fail");
        assert_eq!(err.exit_code, 1);

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn passes_when_no_public_items_or_docs_ok() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_no_public();
        std::env::set_current_dir(&root).expect("chdir");

        let result = run(&[]);
        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
        result.expect("thibitisha should pass when there are no public items to document");
    }

    #[test]
    fn test_coverage_threshold_fails_when_below_target() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_uncovered_public_fn();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&["--kiwango-cha-jaribio".to_string(), "100".to_string()])
            .expect_err("thibitisha should fail below coverage threshold");
        assert_eq!(err.exit_code, 1);
        assert!(err.message.contains("kiwango cha majaribio"));

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn passes_when_local_trait_is_fully_implemented() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_trait_implemented();
        std::env::set_current_dir(&root).expect("chdir");

        let result = run(&[]);
        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
        result.expect("thibitisha should pass when every sifa has an impl");
    }

    #[test]
    fn fails_when_local_trait_has_no_impl() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_trait_unimplemented();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&[]).expect_err("thibitisha should fail when a sifa has zero impls");
        assert_eq!(err.exit_code, 1);
        assert!(err.message.contains("haina utekelezaji wowote"), "{}", err.message);

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fails_when_kiunganishi_param_is_not_ffi_safe() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_unsafe_ffi_signature();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&[]).expect_err("thibitisha should fail on a non-FFI-safe kiunganishi signature");
        assert_eq!(err.exit_code, 1);
        assert!(err.message.contains("salama kwa ABI ya C"), "{}", err.message);

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_coverage_threshold_not_enforced_by_default() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_uncovered_public_fn();
        std::env::set_current_dir(&root).expect("chdir");

        let result = run(&[]);
        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
        result.expect("thibitisha should not enforce coverage without the flag");
    }

    fn temp_project_uncovered_public_fn() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-cov-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n/// Jumlisha namba mbili.\numma kazi jumlisha(a: Namba, b: Namba) -> Namba { rejesha a + b }\n",
        )
        .expect("src");
        dir
    }

    fn temp_project_no_public() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-ok-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n",
        )
        .expect("src");
        dir
    }

    fn temp_project_trait_implemented() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-trait-ok-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\n/// Inayoonyeshwa.\nsifa Inayoonyeshwa { kazi onyesha(self: Self) -> Neno }\n/// Paka.\numbo Paka { jina: Neno }\nshughuli ya Paka kwa Inayoonyeshwa { kazi onyesha(self: Paka) -> Neno { rejesha self.jina } }\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n",
        )
        .expect("src");
        dir
    }

    fn temp_project_trait_unimplemented() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-trait-missing-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\n/// Inayoonyeshwa.\nsifa Inayoonyeshwa { kazi onyesha(self: Self) -> Neno }\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n",
        )
        .expect("src");
        dir
    }

    fn temp_project_unsafe_ffi_signature() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-ffi-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\n#[kiunganishi]\nkazi kutoka_c(x: Orodha<Namba>) -> Namba { rejesha 0 }\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\n",
        )
        .expect("src");
        dir
    }

    fn temp_project() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-thibitisha-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }\numma kazi wazi() -> Tupu { }",
        )
        .expect("src");
        dir
    }
}
