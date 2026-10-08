use crate::commands::CliError;
use crate::pipeline::project::{
    find_workspace_root, load_project_config, read_lockfile, verify_lockfile_integrity, Dependency,
    ProjectConfig,
};
use crate::pipeline::sharti::filter_module_for_target;
use asili_diagnostics::Diagnostic;
use asili_evaluator::{
    emit_asb, execute_tests_with_timeout, load_asb, parse_format, validate_module, TestResult,
};
use asili_lexer::tokenize;
use asili_parser::{
    discover_tests, parse_tokens, semantic_check_with_env_and_modules, Function, Module, Target,
};
use pata_core::InterfaceRegistry;
use pata_core::{
    check_duplicate_imports, dependency_order, find_module_file, merge_for_semantic, resolve_all,
    ResolvedProgram, StdlibEnv,
};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

// Stable content-addressable cache key from name, source, and target (e.g. for single-file
// builds). Target is included so switching --target forces a rebuild instead of silently
// reusing an artifact built for a different target.
pub fn cache_key(name: &str, source: &str, target: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(source.as_bytes());
    hasher.update(target.as_bytes());
    let result = hasher.finalize();
    format!(
        "{:016x}",
        u64::from_be_bytes(result[..8].try_into().unwrap())
    )
}

/// Resolve the effective build target: CLI flag overrides the manifest's `[jenga] lengo`, which
/// overrides the default "native".
pub fn resolve_target(cli_target: Option<&str>, manifest_target: Option<&str>) -> Target {
    Target(
        cli_target
            .or(manifest_target)
            .unwrap_or("native")
            .to_string(),
    )
}

#[derive(Clone, Debug)]
pub struct CompileOutput {
    pub module: Module,
    pub source: String,
    pub source_path: PathBuf,
    pub config: ProjectConfig,
    /// Set for project builds: used for manifest and cache lookup.
    pub input_hash: Option<String>,
    /// True when project build was satisfied from cache (no compile/emit needed).
    pub from_cache: bool,
    /// The build target that was resolved and used to filter `#[sharti(...)]` items.
    pub target: Target,
}

/// Compute content hash of all inputs to a project build (entry + resolved modules + target).
/// Target is included so switching `--target`/`[jenga].lengo` invalidates the build cache
/// instead of silently reusing an artifact filtered for a different target.
pub fn project_input_hash(
    root: &Path,
    entry_path: &Path,
    entry_content: &str,
    program: &ResolvedProgram,
    dependencies: &BTreeMap<String, Dependency>,
    target: &Target,
) -> String {
    let mut h = DefaultHasher::new();
    entry_path.display().to_string().hash(&mut h);
    entry_content.hash(&mut h);
    target.0.hash(&mut h);
    let mut pairs: Vec<(String, String)> = Vec::new();
    // Any fixed order works for a fingerprint; sorted names don't depend on import structure.
    let mut names: Vec<&String> = program.resolved.keys().collect();
    names.sort();
    for name in names {
        let content = if let Some((path, _)) = find_module_file(name.as_str(), root, dependencies) {
            fs::read_to_string(&path).unwrap_or_default()
        } else {
            format!("builtin:{name}")
        };
        pairs.push((name.clone(), content));
    }
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    for (k, v) in &pairs {
        k.hash(&mut h);
        v.hash(&mut h);
    }
    format!("{:016x}", h.finish())
}

pub fn compile_project(root: &Path, cli_target: Option<&str>) -> Result<CompileOutput, CliError> {
    if let Some(_ws) = find_workspace_root(root) {
        // Workspace detected; member compilation handled by workspace logic
    }
    let mut cfg = load_project_config(root)?;
    if let Some(locked) = read_lockfile(root)? {
        cfg.dependencies = locked;
    }
    let mismatches = verify_lockfile_integrity(root)?;
    if !mismatches.is_empty() {
        let names: Vec<String> = mismatches
            .iter()
            .map(|m| {
                format!(
                    "  - {}: pata.lock={} halisi={}",
                    m.name, m.expected, m.actual
                )
            })
            .collect();
        return Err(CliError::new(
            format!(
                "uadilifu wa tegemezi umeshindwa — yaliyomo ya .asili/packages/ hayalingani na pata.lock:\n{}\n\
                 tumia 'pata ongeza' upya kupata toleo sahihi, au thibitisha maudhui ya .asili/packages/ hayajabadilishwa.",
                names.join("\n")
            ),
            1,
        ));
    }
    let target = resolve_target(cli_target, cfg.target.as_deref());
    let source = fs::read_to_string(&cfg.entrypoint).map_err(|e| {
        CliError::new(
            format!("imeshindwa kusoma chanzo {}: {e}", cfg.entrypoint.display()),
            1,
        )
    })?;

    let mut registry = stdlib_registry(root)?;
    let prelude = registry.prelude_env();
    let (entrypoint, program) =
        parse_and_resolve(&source, root, &cfg.dependencies, &mut registry, &target)?;

    let input_hash = project_input_hash(
        root,
        &cfg.entrypoint,
        &source,
        &program,
        &cfg.dependencies,
        &target,
    );
    let target_dir = root.join("kilele");
    let manifest_path = target_dir.join(format!("{}.build.manifest", cfg.name));
    let asb_path = target_dir.join(format!("{}.asb", cfg.name));
    if manifest_path.is_file() && asb_path.is_file() {
        if let Ok(manifest_content) = fs::read_to_string(&manifest_path) {
            let stored = manifest_content
                .lines()
                .find(|l| l.starts_with("hashi_chanzo="))
                .and_then(|l| l.strip_prefix("hashi_chanzo=").map(str::trim));
            if stored == Some(input_hash.as_str()) {
                let bytes = fs::read(&asb_path).map_err(|e| {
                    CliError::new(
                        format!("imeshindwa kusoma cache {}: {e}", asb_path.display()),
                        1,
                    )
                })?;
                if parse_format(&bytes).as_deref() != Some("bytecode") {
                    let module = load_asb(&bytes)
                        .map_err(|e| CliError::new(format!("kuipakia asb: {e}"), 1))?;
                    return Ok(CompileOutput {
                        module,
                        source,
                        source_path: cfg.entrypoint.clone(),
                        config: cfg,
                        input_hash: Some(input_hash),
                        from_cache: true,
                        target,
                    });
                }
            }
        }
    }

    check_program(&entrypoint, &program, &prelude, true)?;
    validate_module(&program.merged_for_eval).map_err(|errors| diag_err("kitekelezi", errors))?;

    Ok(CompileOutput {
        module: program.merged_for_eval,
        source,
        source_path: cfg.entrypoint.clone(),
        config: cfg,
        input_hash: Some(input_hash),
        from_cache: false,
        target,
    })
}

/// A registry with the builtin modules and `lib/std` interfaces loaded.
fn stdlib_registry(root: &Path) -> Result<InterfaceRegistry, CliError> {
    let mut registry = InterfaceRegistry::new(root.to_path_buf());
    registry.register_builtins();
    registry.load_stdlib()?;
    Ok(registry)
}

/// Lex, parse and resolve one entry source, then drop every item not built for `target` —
/// the front half of every compile, test-discovery and test-listing path.
fn parse_and_resolve(
    source: &str,
    root: &Path,
    dependencies: &BTreeMap<String, Dependency>,
    registry: &mut InterfaceRegistry,
    target: &Target,
) -> Result<(Module, ResolvedProgram), CliError> {
    let tokens = tokenize(source).map_err(|errors| diag_err("leksika", errors))?;
    let mut entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("uchanganuzi", errors))?;
    filter_module_for_target(&mut entrypoint, target)
        .map_err(|errors| diag_err("sharti", errors))?;
    let mut program = resolve_all(&entrypoint, root, dependencies, registry)
        .map_err(|errors| diag_err("utatuzi", errors))?;
    for res in program.resolved.values_mut() {
        filter_module_for_target(&mut res.module, target)
            .map_err(|errors| diag_err("sharti", errors))?;
    }
    filter_module_for_target(&mut program.merged_for_eval, target)
        .map_err(|errors| diag_err("sharti", errors))?;
    Ok((entrypoint, program))
}

/// Semantic-check a resolved program: duplicate imports, every dependency in dependency order,
/// then the merged entry module (`require_main` for programs, not for test discovery).
fn check_program(
    entrypoint: &Module,
    program: &ResolvedProgram,
    prelude: &StdlibEnv,
    require_main: bool,
) -> Result<(), CliError> {
    let dup_errors = check_duplicate_imports(entrypoint, &program.resolved, prelude);
    if !dup_errors.is_empty() {
        return Err(diag_err("semantiki", dup_errors));
    }
    // Modules the resolver already confirmed exist (project-local files, path/vendored
    // dependencies) are allowed `leta` targets alongside the builtin-module whitelist.
    let resolved_modules: HashSet<String> = program.resolved.keys().cloned().collect();
    let order = dependency_order(&program.resolved).map_err(|d| diag_err("utatuzi", vec![d]))?;
    for name in order {
        let res = &program.resolved[&name];
        let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, prelude);
        semantic_check_with_env_and_modules(
            &res.module,
            false,
            ext_fns,
            ext_consts,
            resolved_modules.clone(),
        )
        .map_err(|errors| diag_err("semantiki", errors))?;
    }
    let (merged_fns, merged_consts) = merge_for_semantic(entrypoint, &program.resolved, prelude);
    // Check against the merged module (not the bare entrypoint): merge_for_eval already folds
    // imported public structs/traits/impls/functions in, so struct-literal/method-dispatch checks
    // (which only look at `self.module.*`, not the extern maps) can see cross-module types.
    semantic_check_with_env_and_modules(
        &program.merged_for_eval,
        require_main,
        merged_fns,
        merged_consts,
        resolved_modules,
    )
    .map_err(|errors| diag_err("semantiki", errors))
}

/// Compile a single .as file without pata.toml. Root for resolve/stdlib is the file's parent.
pub fn compile_single_file(
    entry_path: &Path,
    cli_target: Option<&str>,
) -> Result<CompileOutput, CliError> {
    let target = resolve_target(cli_target, None);
    let root = entry_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let source = fs::read_to_string(entry_path).map_err(|e| {
        CliError::new(
            format!("imeshindwa kusoma chanzo {}: {e}", entry_path.display()),
            1,
        )
    })?;
    let name = entry_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("script")
        .to_string();
    let config = ProjectConfig {
        name: name.clone(),
        version: "0.1.0".to_string(),
        asili_version: "1.1".to_string(),
        entrypoint: entry_path.to_path_buf(),
        dependencies: BTreeMap::new(),
        target: None,
        eneo_kazi: None,
    };

    let mut registry = stdlib_registry(&root)?;
    let prelude = registry.prelude_env();
    let (entrypoint, program) =
        parse_and_resolve(&source, &root, &config.dependencies, &mut registry, &target)?;

    check_program(&entrypoint, &program, &prelude, true)?;
    validate_module(&program.merged_for_eval).map_err(|errors| diag_err("kitekelezi", errors))?;

    Ok(CompileOutput {
        module: program.merged_for_eval,
        source,
        source_path: config.entrypoint.clone(),
        config,
        input_hash: None,
        from_cache: false,
        target,
    })
}

/// `pata jenga --namna`: how hard the build tries for native code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BuildProfile {
    /// Best effort: bytecode when the program benefits, native code where the platform has a
    /// native backend, otherwise the artifact runs on the VM (or the tree-walker) with a note.
    #[default]
    Dev,
    /// What ships must run as native code: the program must compile to bytecode and its native
    /// code must be built, or the build fails saying why.
    Release,
}

impl BuildProfile {
    pub fn parse(name: &str) -> Result<Self, CliError> {
        match name {
            "dev" => Ok(BuildProfile::Dev),
            "release" => Ok(BuildProfile::Release),
            "embedded" => Err(CliError::new("namna 'embedded' haijatekelezwa bado", 2)),
            other => Err(CliError::new(
                format!("namna '{other}' haijulikani (tumia dev au release)"),
                2,
            )),
        }
    }
}

/// Ahead-of-time compile a bytecode artifact to native machine code next to it
/// (`<name>.nguvu`, built in-house with no external tools). In `Dev` a platform without a
/// native backend only means the artifact runs on the VM; `Release` fails instead.
/// A release build's standalone executable `<target>/<name>` (`.exe` on Windows): the static
/// runner `tenda` with the artifact and its native image appended
/// (`asili_evaluator::bundle`), which runs directly. The runner comes from `ASILI_TENDA` or sits
/// beside `pata`; without one the build only notes that the executable was skipped.
fn write_standalone(asb: &[u8], target: &Path, name: &str) -> Result<(), CliError> {
    let exe = format!("tenda{}", std::env::consts::EXE_SUFFIX);
    let runner = std::env::var_os("ASILI_TENDA")
        .map(PathBuf::from)
        .or_else(|| Some(std::env::current_exe().ok()?.parent()?.join(&exe)))
        .filter(|p| p.is_file());
    let Some(runner) = runner else {
        println!(
            "programu huru haikujengwa: `{exe}` haipatikani kando ya pata (au weka ASILI_TENDA)"
        );
        return Ok(());
    };
    let runner = fs::read(&runner)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", runner.display()), 1))?;
    let image = fs::read(target.join(asili_evaluator::nguvu::image_file_name(name))).ok();
    let path = target.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    fs::write(
        &path,
        asili_evaluator::bundle::assemble(&runner, asb, image.as_deref()),
    )
    .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", path.display()), 1))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
    }
    println!("programu huru: {}", path.display());
    Ok(())
}

fn build_native_library(
    asb: &[u8],
    target: &Path,
    name: &str,
    profile: BuildProfile,
) -> Result<(), CliError> {
    use asili_evaluator::nguvu;
    let stale = target.join(nguvu::image_file_name(name));
    // Libraries from the retired LLVM backend would only confuse; nothing loads them now.
    let _ = fs::remove_file(target.join(format!("{name}.{}", std::env::consts::DLL_EXTENSION)));
    let _ = fs::remove_file(target.join(format!("{name}.ll")));
    if parse_format(asb).as_deref() != Some("bytecode") {
        let _ = fs::remove_file(&stale);
        return Ok(());
    }
    let program = asili_evaluator::load_asb_bytecode(asb)
        .map_err(|e| CliError::new(format!("kuipakia bytecode: {e}"), 1))?;
    let failure = if !asili_evaluator::aot::enabled() {
        "ASILI_AOT=0".to_string()
    } else if !nguvu::supported() {
        "mfumo huu bado hauungwi mkono".to_string()
    } else {
        match nguvu::write_image(&program, target, name) {
            Ok(path) => {
                println!("msimbo asilia: {}", path.display());
                return Ok(());
            }
            Err(why) => why,
        }
    };
    let _ = fs::remove_file(&stale);
    if profile == BuildProfile::Release {
        return Err(CliError::new(
            format!("--namna release inahitaji msimbo asilia, lakini haukujengwa: {failure}"),
            1,
        ));
    }
    println!("msimbo asilia haukujengwa ({failure}); kilele kitaendeshwa na VM");
    Ok(())
}

pub fn emit_build_artifacts(
    root: &Path,
    compiled: &CompileOutput,
    out_dir: Option<&Path>,
    profile: BuildProfile,
) -> Result<PathBuf, CliError> {
    let target = out_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.join("kilele"));
    fs::create_dir_all(&target)
        .map_err(|e| CliError::new(format!("imeshindwa kuunda {}: {e}", target.display()), 1))?;

    let asb = match profile {
        BuildProfile::Dev => emit_asb(&compiled.module, &compiled.source),
        BuildProfile::Release => asili_evaluator::emit_asb_bytecode(&compiled.module, &compiled.source)
            .map_err(|blocked| {
                CliError::new(
                    format!(
                        "--namna release inahitaji bytecode: {blocked} bado haiwezi kugeuzwa kuwa bytecode"
                    ),
                    1,
                )
            })?,
    };
    let artifact = target.join(format!("{}.asb", compiled.config.name));
    fs::write(&artifact, &asb).map_err(|e| {
        CliError::new(
            format!("imeshindwa kuandika {}: {e}", artifact.display()),
            1,
        )
    })?;

    build_native_library(&asb, &target, &compiled.config.name, profile)?;
    if profile == BuildProfile::Release {
        write_standalone(&asb, &target, &compiled.config.name)?;
    }

    let meta = target.join(format!("{}.build.manifest", compiled.config.name));
    let input_hash_line = compiled
        .input_hash
        .as_deref()
        .map(|h| format!("hashi_chanzo={}\n", h))
        .unwrap_or_default();
    let manifest = format!(
        "mradi={}\nkuingia={}\nkazi={}\nkilele={}.asb\n{}",
        compiled.config.name,
        compiled.source_path.display(),
        compiled.module.functions.len(),
        compiled.config.name,
        input_hash_line
    );
    fs::write(&meta, manifest)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", meta.display()), 1))?;

    let cache_dir = target.join(".asb-cache");
    if let Ok(()) = fs::create_dir_all(&cache_dir) {
        let key = cache_key(&compiled.config.name, &compiled.source, &compiled.target.0);
        let cache_artifact = cache_dir.join(format!("{key}.asb"));
        let _ = fs::write(&cache_artifact, &asb);
    }

    Ok(artifact)
}

/// Discover every `#[jaribio]` test in the project (compile, resolve, semantic-check every
/// source file, same as `run_project_tests_parallel`'s own first half) without running any of
/// them — shared by the normal pass/fail runner and the coverage-mode runner below, so the
/// resolve/semantic-check logic exists in exactly one place.
fn discover_project_tests(
    root: &Path,
    filter: Option<&str>,
) -> Result<Vec<(Module, Function)>, CliError> {
    let cfg = load_project_config(root)?;
    let target = resolve_target(None, cfg.target.as_deref());
    let src_dir = cfg
        .entrypoint
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.join("src"));
    let mut registry = stdlib_registry(root)?;
    let prelude = registry.prelude_env();

    let mut modules_and_tests: Vec<(Module, Function)> = Vec::new();
    for file in collect_as_files(&src_dir)? {
        let source = fs::read_to_string(&file)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", file.display()), 1))?;
        let (entrypoint, program) =
            parse_and_resolve(&source, root, &cfg.dependencies, &mut registry, &target)?;
        check_program(&entrypoint, &program, &prelude, false)?;

        for test_fn in discover_tests(&program.merged_for_eval) {
            modules_and_tests.push((program.merged_for_eval.clone(), test_fn));
        }
    }

    if let Some(f) = filter {
        modules_and_tests.retain(|(_, t)| t.name.contains(f));
    }

    Ok(modules_and_tests)
}

/// Run every discovered test with line-level coverage tracking, returning both the pass/fail
/// results (for the normal `pata jaribu` report) and the module's aggregated `CoverageMetrics`
/// (real executed-line data from `asili_evaluator::run_test_with_coverage`, not the old
/// function-name-presence check). Sequential only — coverage tracking allocates a `HashSet` per
/// test, so unlike the plain pass/fail path this doesn't currently have a parallel variant; the
/// tradeoff is acceptable since `--chanjo` is an opt-in diagnostic mode, not the default path.
pub fn run_project_tests_with_coverage(
    root: &Path,
    filter: Option<&str>,
) -> Result<(Vec<TestResult>, crate::pipeline::coverage::CoverageMetrics), CliError> {
    let modules_and_tests = discover_project_tests(root, filter)?;

    if modules_and_tests.is_empty() {
        let metrics = crate::pipeline::coverage::CoverageMetrics {
            total_lines: HashSet::new(),
            executed_lines: HashSet::new(),
            coverage_percent: 0.0,
        };
        return Ok((Vec::new(), metrics));
    }

    let mut results = Vec::new();
    let mut all_executed_lines: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (module, test_fn) in &modules_and_tests {
        let (result, lines) = asili_evaluator::run_test_with_coverage(module, test_fn);
        all_executed_lines.extend(lines);
        results.push(result);
    }

    // Coverage is computed against the union of every test's module (a project usually has one
    // real source module per file, each already merged with its own imports via
    // merged_for_eval) — sum each distinct module's total lines rather than picking one
    // arbitrary module, since a multi-file project's tests are spread across several.
    let mut seen_modules: Vec<&Module> = Vec::new();
    for (module, _) in &modules_and_tests {
        if !seen_modules.iter().any(|m| std::ptr::eq(*m, module)) {
            seen_modules.push(module);
        }
    }
    let mut total_lines = std::collections::HashSet::new();
    for module in &seen_modules {
        total_lines.extend(crate::pipeline::coverage::total_statement_lines(module));
    }
    let executed_lines: std::collections::HashSet<usize> = all_executed_lines
        .into_iter()
        .filter(|l| total_lines.contains(l))
        .collect();
    let coverage_percent = if total_lines.is_empty() {
        0.0
    } else {
        (executed_lines.len() as f64 / total_lines.len() as f64) * 100.0
    };
    let metrics = crate::pipeline::coverage::CoverageMetrics {
        total_lines,
        executed_lines,
        coverage_percent,
    };

    Ok((results, metrics))
}

pub fn run_project_tests_parallel(
    root: &Path,
    filter: Option<&str>,
    fail_fast: bool,
    num_threads: Option<usize>,
    timeout: Option<std::time::Duration>,
) -> Result<Vec<TestResult>, CliError> {
    let modules_and_tests = discover_project_tests(root, filter)?;

    if modules_and_tests.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(threads) = num_threads {
        let results = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .ok()
            .map(|pool| {
                pool.install(|| {
                    modules_and_tests
                        .par_iter()
                        .map(|(m, f)| {
                            let result = execute_tests_with_timeout(
                                &[(m.clone(), f.clone())],
                                false,
                                timeout,
                            );
                            result.into_iter().next().unwrap_or_else(|| TestResult {
                                name: f.name.clone(),
                                passed: false,
                                message: "failed to run test".to_string(),
                            })
                        })
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_else(|| execute_tests_with_timeout(&modules_and_tests, fail_fast, timeout));
        Ok(results)
    } else {
        Ok(execute_tests_with_timeout(
            &modules_and_tests,
            fail_fast,
            timeout,
        ))
    }
}

/// List test names discovered in the project (no execution). For use with `pata jaribu --list`.
pub fn list_project_tests(root: &Path) -> Result<Vec<String>, CliError> {
    Ok(discover_project_tests(root, None)?
        .into_iter()
        .map(|(_, test)| test.name)
        .collect())
}

fn collect_as_files(root: &Path) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    walk(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
    for entry in fs::read_dir(dir)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", dir.display()), 1))?
    {
        let entry = entry.map_err(|e| CliError::new(format!("hitilafu ya kiingilio: {e}"), 1))?;
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.extension().map(|e| e == "as").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}

fn diag_err(stage: &str, diags: Vec<Diagnostic>) -> CliError {
    let mut msg = format!("{stage}: makosa {}\n", diags.len());
    for d in diags {
        let where_ = d
            .span
            .map(|s| format!("{}:{}", s.line, s.column))
            .unwrap_or_else(|| "?".to_string());
        msg.push_str(&format!(
            "- [{}:{}] {} @{}\n",
            d.stage, d.code, d.message, where_
        ));
        if let Some(map) = d.context_map {
            msg.push_str(&format!(
                "  context_map symbol={} created={:?} moved={:?} borrowed={:?} dropped={:?}\n",
                map.symbol, map.created_at, map.moved_at, map.borrowed_at, map.dropped_at
            ));
        }
    }
    CliError::new(msg, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::temp_dir(&format!("compile-{label}"))
    }

    /// Cross-package `leta`: entrypoint imports a struct and a public constant from a path
    /// dependency and uses both. Exercises the resolver's path-dependency lookup and
    /// merge_for_eval's struct/constant handling end-to-end through compile_project.
    #[test]
    fn build_profiles_parse_and_reject_unimplemented_ones() {
        assert_eq!(BuildProfile::parse("dev").unwrap(), BuildProfile::Dev);
        assert_eq!(
            BuildProfile::parse("release").unwrap(),
            BuildProfile::Release
        );
        assert!(BuildProfile::parse("embedded")
            .unwrap_err()
            .message
            .contains("haijatekelezwa"));
        assert!(BuildProfile::parse("haraka")
            .unwrap_err()
            .message
            .contains("haijulikani"));
    }

    /// Release never ships a tree-walker artifact: a program the bytecode compiler can't lower
    /// fails with the construct that blocked it, where dev quietly falls back.
    #[test]
    fn release_profile_refuses_the_ast_fallback() {
        let root = temp_dir("release-ast");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("pata.toml"), crate::test_support::MANIFEST).unwrap();
        // A pattern `weka` isn't lowered to bytecode yet.
        fs::write(
            root.join("src/kuu.as"),
            "kazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    weka (a, b) = jozi(1, 2)\n}\n",
        )
        .unwrap();
        let compiled = compile_project(&root, None).expect("compiles");
        let err = emit_build_artifacts(&root, &compiled, None, BuildProfile::Release)
            .expect_err("release must refuse the AST artifact");
        assert!(
            err.message.contains("inahitaji bytecode"),
            "{}",
            err.message
        );
        assert!(err.message.contains("kazi 'kuu'"), "{}", err.message);
        emit_build_artifacts(&root, &compiled, None, BuildProfile::Dev).expect("dev falls back");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn compile_project_resolves_struct_and_constant_from_path_dependency() {
        let workspace = temp_dir("workspace");
        let dep_dir = workspace.join("mathutil");
        let app_dir = workspace.join("app");
        fs::create_dir_all(dep_dir.join("src")).expect("mkdir dep");
        fs::create_dir_all(app_dir.join("src")).expect("mkdir app");
        fs::create_dir_all(app_dir.join("lib/std")).expect("mkdir lib/std");

        fs::write(
            dep_dir.join("src/mathutil.as"),
            "thabiti PI: Namba = 3.14\numma umbo Punkt { x: Namba, y: Namba }\n",
        )
        .expect("write dep");

        fs::write(
            app_dir.join("pata.toml"),
            format!(
                "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\nmathutil = {{ path = \"{}\" }}\n",
                dep_dir.display()
            ),
        )
        .expect("write manifest");
        fs::write(
            app_dir.join("src/kuu.as"),
            "leta mathutil\nleta matumizi\nkazi thamani() -> Namba {\n  weka p = Punkt { x: PI, y: 1.0 }\n  rejesha p.x\n}\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n  chapisha(\"ok\")\n}",
        )
        .expect("write src");
        fs::write(
            app_dir.join("lib/std/mfumo.asi"),
            "kazi chapisha(ujumbe: Neno) -> Tupu\n",
        )
        .expect("write stdlib stub");

        let compiled = compile_project(&app_dir, None).expect("compile_project ok");
        assert!(compiled.module.structs.iter().any(|s| s.name == "Punkt"));
        assert!(compiled.module.constants.iter().any(|c| c.name == "PI"));

        // Prove it's not just structurally present but actually runs end-to-end: a struct field
        // initialized from an imported constant, through the CLI's full compile pipeline and the
        // evaluator's runtime, should produce the constant's real value.
        let result = asili_evaluator::run_function(&compiled.module, "thamani", vec![])
            .expect("thamani() should run using the imported struct and constant");
        match result {
            asili_evaluator::Value::Namba(n) => {
                assert!((n - 3.14).abs() < 1e-9, "expected PI (3.14), got {n}")
            }
            other => panic!("expected Namba(3.14), got {other:?}"),
        }

        let _ = fs::remove_dir_all(&workspace);
    }

    /// A version dependency (no `path`) resolves against the vendored `.asili/packages/<name>`
    /// cache when present there, instead of only ever failing with RES002.
    #[test]
    fn find_module_file_resolves_vendored_version_dependency() {
        let root = temp_dir("vendored");
        let pkg_src = root.join(".asili/packages/greeter/src");
        fs::create_dir_all(&pkg_src).expect("mkdir vendored pkg");
        fs::write(
            pkg_src.join("greeter.as"),
            "umma kazi salamu() -> Neno { rejesha \"hi\" }\n",
        )
        .expect("write vendored module");

        let mut deps = BTreeMap::new();
        deps.insert(
            "greeter".to_string(),
            Dependency::Version("^1.0".to_string()),
        );

        let found = find_module_file("greeter", &root, &deps);
        assert!(found.is_some(), "expected vendored greeter.as to resolve");
        let (path, is_asi) = found.unwrap();
        assert!(!is_asi);
        assert_eq!(path, pkg_src.join("greeter.as"));

        let _ = fs::remove_dir_all(&root);
    }

    /// The lockfile-integrity floor: `compile_project` must re-hash a vendored version
    /// dependency and compare against `pata.lock`'s recorded checksum before building. A vendored
    /// directory whose content still matches the lock builds fine; one that's been tampered with
    /// after the fact (e.g. someone hand-edits `.asili/packages/<name>/` post-fetch) must fail
    /// loudly instead of silently compiling against altered code.
    #[test]
    fn compile_project_rejects_tampered_vendored_dependency() {
        let root = temp_dir("integrity");
        let pkg_dir = root.join(".asili/packages/greeter");
        fs::create_dir_all(pkg_dir.join("src")).expect("mkdir vendored pkg");
        fs::create_dir_all(root.join("src")).expect("mkdir src");
        fs::create_dir_all(root.join("lib/std")).expect("mkdir lib/std");
        fs::write(
            pkg_dir.join("src/greeter.as"),
            "umma kazi salamu() -> Neno { rejesha \"hi\" }\n",
        )
        .expect("write vendored module");

        let real_hash = pata_package::hash_dir(&pkg_dir).expect("hash vendored dir");
        let lock_toml = format!(
            "version = \"1\"\nlocked_at = \"2026-01-01T00:00:00Z\"\n\n[dependencies.greeter]\nversion = \"1.0.0\"\nchecksum = \"{real_hash}\"\nsource = \"registry\"\n"
        );
        fs::write(root.join("pata.lock"), lock_toml).expect("write pata.lock");

        fs::write(
            root.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\ngreeter = \"^1.0\"\n",
        )
        .expect("write manifest");
        fs::write(
            root.join("src/kuu.as"),
            "leta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n  chapisha(\"ok\")\n}",
        )
        .expect("write src");
        fs::write(
            root.join("lib/std/mfumo.asi"),
            "kazi chapisha(ujumbe: Neno) -> Tupu\n",
        )
        .expect("write stdlib stub");

        // Content still matches the lock: build succeeds.
        compile_project(&root, None)
            .expect("compile_project ok when vendored content matches pata.lock");

        // Tamper with the vendored content after the fact — the lock still says `real_hash`.
        fs::write(
            pkg_dir.join("src/greeter.as"),
            "umma kazi salamu() -> Neno { rejesha \"tampered\" }\n",
        )
        .expect("tamper with vendored module");

        let err = compile_project(&root, None)
            .expect_err("compile_project must reject content that no longer matches pata.lock");
        assert!(
            err.message.contains("uadilifu") && err.message.contains("greeter"),
            "expected an integrity-mismatch error naming 'greeter', got: {}",
            err.message
        );

        let _ = fs::remove_dir_all(&root);
    }

    /// End-to-end: `[jenga] lengo = "wasm"` in pata.toml filters out a
    /// `#[sharti(lengo="native")]`-gated function before it ever reaches the emitted `.asb` —
    /// not just at the in-memory `Module` level (compile_project_*), but through the full
    /// jenga pipeline including emit_build_artifacts + load_asb deserialization.
    #[test]
    fn sharti_gated_function_absent_from_emitted_asb_for_non_matching_target() {
        let root = temp_dir("sharti-asb");
        fs::create_dir_all(root.join("src")).expect("mkdir src");
        fs::create_dir_all(root.join("lib/std")).expect("mkdir lib/std");
        fs::write(
            root.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[jenga]\nlengo = \"wasm\"\n\n[tegemezi]\n",
        )
        .expect("write manifest");
        fs::write(
            root.join("src/kuu.as"),
            "leta matumizi\n#[sharti(lengo = \"native\")]\nkazi tu_native() -> Tupu {\n  rejesha\n}\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n  chapisha(\"ok\")\n}",
        )
        .expect("write src");
        fs::write(
            root.join("lib/std/mfumo.asi"),
            "kazi chapisha(ujumbe: Neno) -> Tupu\n",
        )
        .expect("write stdlib stub");

        let compiled =
            compile_project(&root, None).expect("compile_project ok (target=wasm from manifest)");
        assert_eq!(compiled.target, Target("wasm".to_string()));
        assert!(
            !compiled
                .module
                .functions
                .iter()
                .any(|f| f.name == "tu_native"),
            "tu_native should be filtered out of the in-memory module for target=wasm"
        );

        let artifact =
            emit_build_artifacts(&root, &compiled, None, BuildProfile::Dev).expect("emit .asb");
        let bytes = fs::read(&artifact).expect("read .asb");
        // Bytecode when the program lowers to it, else the serialized AST.
        let names: Vec<String> = if parse_format(&bytes).as_deref() == Some("bytecode") {
            asili_evaluator::load_asb_bytecode(&bytes)
                .expect("load_asb_bytecode")
                .functions
                .into_iter()
                .map(|f| f.name)
                .collect()
        } else {
            load_asb(&bytes)
                .expect("load_asb")
                .functions
                .into_iter()
                .map(|f| f.name)
                .collect()
        };
        assert!(
            !names.iter().any(|n| n == "tu_native"),
            "tu_native should be absent from the emitted .asb, not just the in-memory module"
        );
        assert!(names.iter().any(|n| n == "kuu"));

        let _ = fs::remove_dir_all(&root);
    }

    /// Builds every real example project under this repo's `examples/` directory and asserts
    /// zero diagnostics — `compile_project` returning `Ok` means lex/parse/resolve/semantic-check
    /// all succeeded cleanly, since any failure there is surfaced as an `Err(CliError)` (see
    /// `diag_err`). This is the actual `pata/cli` integration-test gap
    /// `docs/design/pata-production-readiness.md` item 5 named: `pata`'s own test suite had no
    /// test that builds every example, so a regression in the resolver/semantic-checker/
    /// formatter layer `pata` depends on could ship silently, caught only by a human running
    /// `pata jenga` by hand across the example set. Runs in this crate's own suite (not a
    /// separate `pata/cli/tests/*.rs` integration test) because `pata-cli` is a binary-only
    /// crate with no `[lib]` target — an external integration test has no way to call
    /// `compile_project` at all.
    ///
    /// Every example directory containing a `pata.toml` at its own root is built directly;
    /// `examples/cross_package` is the one exception (its real project root is the nested
    /// `app/` subdirectory, with `mathutil/` as a sibling path dependency — see its own
    /// `pata.toml`), discovered by walking one level deeper when no `pata.toml` exists at the
    /// example's own top level.
    #[test]
    fn every_example_project_builds_with_zero_diagnostics() {
        let examples_root = examples_dir();
        let mut project_roots: Vec<PathBuf> = Vec::new();

        for entry in fs::read_dir(&examples_root).expect("read examples/ dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if !path.is_dir() {
                continue; // skip loose files like test_syntax.as
            }
            if path.join("pata.toml").is_file() {
                project_roots.push(path);
                continue;
            }
            // No pata.toml at this level -- check one level deeper (cross_package/app/).
            let Ok(sub_entries) = fs::read_dir(&path) else {
                continue;
            };
            for sub_entry in sub_entries.flatten() {
                let sub_path = sub_entry.path();
                if sub_path.is_dir() && sub_path.join("pata.toml").is_file() {
                    project_roots.push(sub_path);
                }
            }
        }

        assert!(
            project_roots.len() >= 15,
            "expected at least 15 example projects to be discovered under {}, found {} -- the \
             discovery logic itself may be broken, not the examples",
            examples_root.display(),
            project_roots.len()
        );

        let mut failures = Vec::new();
        for root in &project_roots {
            if let Err(e) = compile_project(root, None) {
                failures.push(format!("{}: {}", root.display(), e.message));
            }
        }

        assert!(
            failures.is_empty(),
            "the following example project(s) failed to build cleanly:\n{}",
            failures.join("\n")
        );
    }

    /// Locate this repo's `examples/` directory from `CARGO_MANIFEST_DIR` (which points at
    /// `pata/cli/` when this test is compiled) — two levels up, then into `examples/`.
    fn examples_dir() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent() // pata/
            .and_then(|p| p.parent()) // repo root
            .expect("pata/cli should be two levels under the repo root")
            .join("examples")
    }
}
