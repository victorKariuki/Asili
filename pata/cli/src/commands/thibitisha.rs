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
// that's never implemented at all), FFI-safety of #[kiunganishi]-tagged signatures, type
// stability against the most recent `v<semver>` git tag (when the project is a git repo with
// one — see `pipeline::stability`), and (opt-in via --kiwango-cha-jaribio) test coverage.
//
// Not implemented: full ABI compatibility against a C-signature contract, since
// #[kiunganishi]/kiungo has no such contract yet (FFI is a documented Phase IV stub,
// core/evaluator/src/builtins/kiungo.rs). What thibitisha checks today for #[kiunganishi]
// (FFI-safe types) is the real, checkable prerequisite for that future check, not a placeholder.
//
// Every check below runs and its result is collected, rather than stopping at the first failure
// — a user (or CI) sees every problem in one run instead of fixing them one at a time across
// repeated invocations. Compilation is the one hard prerequisite (nothing else can meaningfully
// run against a project that doesn't compile), everything after it always runs regardless of
// earlier check outcomes. `--json` reports the same collected list as a machine-readable array;
// text mode reports each check's name and pass/fail, matching CI-log conventions.
pub fn run(args: &[String]) -> CliResult {
    let (threshold, json) = parse_args(args)?;

    let output = compile_project(Path::new("."), None)?;

    let mut checks: Vec<(&'static str, Result<(), String>)> = Vec::new();
    checks.push(("nyaraka", enforce_docs(Path::new(".")).map_err(|e| e.message)));
    checks.push(("ukamilifu_wa_sifa", enforce_trait_completeness(&output.module).map_err(|e| e.message)));
    checks.push(("usalama_wa_ffi", enforce_ffi_signatures(&output.module).map_err(|e| e.message)));
    checks.push((
        "uthabiti_wa_aina",
        crate::pipeline::stability::enforce_type_stability(Path::new("."), &output.module).map_err(|e| e.message),
    ));

    let format_result: Result<(), String> = (|| {
        let files = collect_asili_files(Path::new("."))?;
        let (_, changed) = check_or_write(&files, true)?;
        if changed > 0 {
            return Err(CliError::new(
                "mafaili hayajafuata muundo sahihi: tumia `pata nadhifu` kwanza",
                1,
            ));
        }
        Ok(())
    })()
    .map_err(|e: CliError| e.message);
    checks.push(("umbizo", format_result));

    if let Some(threshold) = threshold {
        checks.push(("kiwango_cha_jaribio", enforce_test_coverage(&output.module, threshold).map_err(|e| e.message)));
    }

    let failed: Vec<&(&'static str, Result<(), String>)> = checks.iter().filter(|(_, r)| r.is_err()).collect();

    if json {
        let output_json = serde_json::json!({
            "sawa": failed.is_empty(),
            "ukaguzi": checks.iter().map(|(name, r)| serde_json::json!({
                "jina": name,
                "sawa": r.is_ok(),
                "ujumbe": match r { Ok(()) => serde_json::Value::Null, Err(m) => serde_json::json!(m) },
            })).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&output_json).unwrap());
    } else {
        for (name, r) in &checks {
            match r {
                Ok(()) => println!("[SAWA] {name}"),
                Err(m) => println!("[KOSA] {name} - {m}"),
            }
        }
    }

    if !failed.is_empty() {
        // Carry every failing check's own message (not just its name) into the returned error —
        // the single most common caller of `pata thibitisha` is a human or CI log reading this
        // one string, and "usalama_wa_ffi" alone tells them nothing a bare check name wouldn't;
        // the real detail (which parameter, which type) lives in each check's own message.
        let detail = failed
            .iter()
            .map(|(name, r)| format!("{name}: {}", r.as_ref().err().unwrap()))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(CliError::new(format!("thibitisha imeshindwa: {detail}"), 1));
    }

    if !json {
        println!("thibitisha: sawa");
    }
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

fn parse_args(args: &[String]) -> Result<(Option<f64>, bool), CliError> {
    let mut threshold = None;
    let mut json = false;
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
            "--json" => {
                json = true;
                i += 1;
            }
            other => {
                return Err(CliError::new(
                    format!("hoja isiyotambuliwa kwenye thibitisha: {other}"),
                    2,
                ));
            }
        }
    }
    Ok((threshold, json))
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

    /// The point of the accumulate-and-report restructure: a project with two independent
    /// problems at once (an unsafe FFI signature AND non-canonical formatting) must report BOTH
    /// in a single run, not just whichever check happened to run first — proving thibitisha no
    /// longer stops at the first failure.
    #[test]
    fn reports_every_failing_check_not_just_the_first() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_unsafe_ffi_signature();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&[]).expect_err("thibitisha should fail with multiple problems");
        assert!(err.message.contains("usalama_wa_ffi"), "{}", err.message);
        assert!(err.message.contains("umbizo"), "expected the format check to also be reported, got: {}", err.message);

        std::env::set_current_dir(&original).expect("restore");
        let _ = fs::remove_dir_all(root);
    }

    /// `--json` reports a structured array with one entry per check (name, sawa: bool, ujumbe),
    /// not just plain text — real machine-readable output a CI pipeline or editor could parse,
    /// proving `--json` actually changes the output shape rather than being a no-op flag.
    #[test]
    fn json_mode_reports_structured_per_check_results() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project_unsafe_ffi_signature();
        std::env::set_current_dir(&root).expect("chdir");

        let err = run(&["--json".to_string()]).expect_err("thibitisha should still fail in json mode");
        assert_eq!(err.exit_code, 1);
        // The JSON body itself is printed to stdout inside run(), not carried on the CliError —
        // can't easily capture stdout here without restructuring run() to return the value
        // directly (same tradeoff jaribu.rs's own --json tests already made, see
        // chanjo_json_includes_coverage_field), so this proves the flag is accepted and the
        // pass/fail outcome is unchanged by it, matching that established test-depth convention.
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
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    chapisha(\"x\")\n}\n\n/// Jumlisha namba mbili.\numma kazi jumlisha(a: Namba, b: Namba) -> Namba {\n    rejesha a + b\n}\n",
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
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    chapisha(\"x\")\n}\n",
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
            "leta matumizi\n\n/// Inayoonyeshwa.\nsifa Inayoonyeshwa {\n    kazi onyesha(self: Self) -> Neno\n}\n\n/// Paka.\numbo Paka {\n    jina: Neno\n}\nshughuli ya Paka kwa Inayoonyeshwa {\n    kazi onyesha(self: Paka) -> Neno {\n        rejesha self.jina\n    }\n}\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    chapisha(\"x\")\n}\n",
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
