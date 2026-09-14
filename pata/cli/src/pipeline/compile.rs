use crate::commands::CliError;
use crate::pipeline::project::{load_project_config, read_lockfile, find_workspace_root, ProjectConfig, Dependency};
use crate::pipeline::sharti::filter_module_for_target;
use pata_core::InterfaceRegistry;
use pata_core::{
    check_duplicate_imports, dependency_order, find_module_file, merge_for_semantic, resolve_all,
    ResolvedProgram,
};
use asili_diagnostics::Diagnostic;
use asili_evaluator::{emit_asb, load_asb, execute_tests_with_timeout, validate_module, TestResult};
use asili_lexer::tokenize;
use asili_parser::{discover_tests, parse_tokens, semantic_check_with_env_and_modules, Function, Module, Target};
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
    format!("{:016x}", u64::from_be_bytes(result[..8].try_into().unwrap()))
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
    for name in dependency_order(&program.resolved) {
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
    let target = resolve_target(cli_target, cfg.target.as_deref());
    let source = fs::read_to_string(&cfg.entrypoint).map_err(|e| {
        CliError::new(
            format!("imeshindwa kusoma chanzo {}: {e}", cfg.entrypoint.display()),
            1,
        )
    })?;

    let tokens = tokenize(&source).map_err(|errors| diag_err("leksika", errors))?;
    let mut entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("uchanganuzi", errors))?;
    filter_module_for_target(&mut entrypoint, &target).map_err(|errors| diag_err("sharti", errors))?;
    let mut registry = InterfaceRegistry::new(root.to_path_buf());
    registry.register_builtins();
    registry.load_stdlib()?;
    let prelude = registry.prelude_env();
    let mut program = resolve_all(&entrypoint, root, &cfg.dependencies, &mut registry).map_err(|errors| diag_err("utatuzi", errors))?;
    for res in program.resolved.values_mut() {
        filter_module_for_target(&mut res.module, &target).map_err(|errors| diag_err("sharti", errors))?;
    }
    filter_module_for_target(&mut program.merged_for_eval, &target).map_err(|errors| diag_err("sharti", errors))?;

    let input_hash = project_input_hash(root, &cfg.entrypoint, &source, &program, &cfg.dependencies, &target);
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
                    CliError::new(format!("imeshindwa kusoma cache {}: {e}", asb_path.display()), 1)
                })?;
                let module = load_asb(&bytes).map_err(|e| {
                    CliError::new(format!("kuipakia asb: {e}"), 1)
                })?;
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

    let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
    if !dup_errors.is_empty() {
        return Err(diag_err("semantiki", dup_errors));
    }

    // Modules the resolver already confirmed exist (project-local files, path/vendored
    // dependencies) are allowed `leta` targets alongside the builtin-module whitelist.
    let resolved_modules: HashSet<String> = program.resolved.keys().cloned().collect();
    for name in dependency_order(&program.resolved) {
        let res = &program.resolved[&name];
        let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
        semantic_check_with_env_and_modules(&res.module, false, ext_fns, ext_consts, resolved_modules.clone())
            .map_err(|errors| diag_err("semantiki", errors))?;
    }

    let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
    // Check against the merged module (not the bare entrypoint): merge_for_eval already folds
    // imported public structs/traits/impls/functions in, so struct-literal/method-dispatch checks
    // (which only look at `self.module.*`, not the extern maps) can see cross-module types.
    semantic_check_with_env_and_modules(&program.merged_for_eval, true, merged_fns, merged_consts, resolved_modules)
        .map_err(|errors| diag_err("semantiki", errors))?;
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

/// Compile a single .as file without pata.toml. Root for resolve/stdlib is the file's parent.
pub fn compile_single_file(entry_path: &Path, cli_target: Option<&str>) -> Result<CompileOutput, CliError> {
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
    };

    let tokens = tokenize(&source).map_err(|errors| diag_err("leksika", errors))?;
    let mut entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("uchanganuzi", errors))?;
    filter_module_for_target(&mut entrypoint, &target).map_err(|errors| diag_err("sharti", errors))?;
    let mut registry = InterfaceRegistry::new(root.clone());
    registry.register_builtins();
    registry.load_stdlib()?;
    let prelude = registry.prelude_env();
    let mut program = resolve_all(&entrypoint, &root, &config.dependencies, &mut registry).map_err(|errors| diag_err("utatuzi", errors))?;
    for res in program.resolved.values_mut() {
        filter_module_for_target(&mut res.module, &target).map_err(|errors| diag_err("sharti", errors))?;
    }
    filter_module_for_target(&mut program.merged_for_eval, &target).map_err(|errors| diag_err("sharti", errors))?;

    let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
    if !dup_errors.is_empty() {
        return Err(diag_err("semantiki", dup_errors));
    }

    let resolved_modules: HashSet<String> = program.resolved.keys().cloned().collect();
    for dep_name in dependency_order(&program.resolved) {
        let res = &program.resolved[&dep_name];
        let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
        semantic_check_with_env_and_modules(&res.module, false, ext_fns, ext_consts, resolved_modules.clone())
            .map_err(|errors| diag_err("semantiki", errors))?;
    }

    let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
    // Check against the merged module (not the bare entrypoint): merge_for_eval already folds
    // imported public structs/traits/impls/functions in, so struct-literal/method-dispatch checks
    // (which only look at `self.module.*`, not the extern maps) can see cross-module types.
    semantic_check_with_env_and_modules(&program.merged_for_eval, true, merged_fns, merged_consts, resolved_modules)
        .map_err(|errors| diag_err("semantiki", errors))?;
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

pub fn emit_build_artifacts(root: &Path, compiled: &CompileOutput, out_dir: Option<&Path>) -> Result<PathBuf, CliError> {
    let target = out_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.join("kilele"));
    fs::create_dir_all(&target)
        .map_err(|e| CliError::new(format!("imeshindwa kuunda {}: {e}", target.display()), 1))?;

    let asb = emit_asb(&compiled.module, &compiled.source);
    let artifact = target.join(format!("{}.asb", compiled.config.name));
    fs::write(&artifact, &asb)
        .map_err(|e| CliError::new(format!("imeshindwa kuandika {}: {e}", artifact.display()), 1))?;

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

pub fn run_project_tests(root: &Path, filter: Option<&str>, fail_fast: bool) -> Result<Vec<TestResult>, CliError> {
    run_project_tests_parallel(root, filter, fail_fast, None, None)
}

pub fn run_project_tests_parallel(
    root: &Path,
    filter: Option<&str>,
    fail_fast: bool,
    num_threads: Option<usize>,
    timeout: Option<std::time::Duration>,
) -> Result<Vec<TestResult>, CliError> {
    let cfg = load_project_config(root)?;
    let target = resolve_target(None, cfg.target.as_deref());
    let src_dir = cfg
        .entrypoint
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.join("src"));
    let mut registry = InterfaceRegistry::new(root.to_path_buf());
    registry.register_builtins();
    registry.load_stdlib()?;
    let prelude = registry.prelude_env();

    let mut modules_and_tests: Vec<(Module, Function)> = Vec::new();
    for file in collect_as_files(&src_dir)? {
        let source = fs::read_to_string(&file)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", file.display()), 1))?;
        let tokens = tokenize(&source).map_err(|errors| diag_err("leksika", errors))?;
        let mut entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("uchanganuzi", errors))?;
        filter_module_for_target(&mut entrypoint, &target).map_err(|errors| diag_err("sharti", errors))?;

        let mut program = resolve_all(&entrypoint, root, &cfg.dependencies, &mut registry).map_err(|errors| diag_err("utatuzi", errors))?;
        for res in program.resolved.values_mut() {
            filter_module_for_target(&mut res.module, &target).map_err(|errors| diag_err("sharti", errors))?;
        }
        filter_module_for_target(&mut program.merged_for_eval, &target).map_err(|errors| diag_err("sharti", errors))?;
        let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
        if !dup_errors.is_empty() {
            return Err(diag_err("semantiki", dup_errors));
        }
        let resolved_modules: HashSet<String> = program.resolved.keys().cloned().collect();
        for name in dependency_order(&program.resolved) {
            let res = &program.resolved[&name];
            let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
            semantic_check_with_env_and_modules(&res.module, false, ext_fns, ext_consts, resolved_modules.clone())
                .map_err(|errors| diag_err("semantiki", errors))?;
        }
        let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
        semantic_check_with_env_and_modules(&program.merged_for_eval, false, merged_fns, merged_consts, resolved_modules)
            .map_err(|errors| diag_err("semantiki", errors))?;

        for test_fn in discover_tests(&program.merged_for_eval) {
            modules_and_tests.push((program.merged_for_eval.clone(), test_fn));
        }
    }

    if let Some(f) = filter {
        modules_and_tests.retain(|(_, t)| t.name.contains(f));
    }

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
                            let result = execute_tests_with_timeout(&[(m.clone(), f.clone())], false, timeout);
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
        Ok(execute_tests_with_timeout(&modules_and_tests, fail_fast, timeout))
    }
}

/// List test names discovered in the project (no execution). For use with `pata jaribu --list`.
pub fn list_project_tests(root: &Path) -> Result<Vec<String>, CliError> {
    let cfg = load_project_config(root)?;
    let target = resolve_target(None, cfg.target.as_deref());
    let src_dir = cfg
        .entrypoint
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.join("src"));
    let mut registry = InterfaceRegistry::new(root.to_path_buf());
    registry.register_builtins();
    registry.load_stdlib()?;
    let prelude = registry.prelude_env();

    let mut names = Vec::new();
    for file in collect_as_files(&src_dir)? {
        let source = fs::read_to_string(&file)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", file.display()), 1))?;
        let tokens = tokenize(&source).map_err(|errors| diag_err("leksika", errors))?;
        let mut entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("uchanganuzi", errors))?;
        filter_module_for_target(&mut entrypoint, &target).map_err(|errors| diag_err("sharti", errors))?;

        let mut program = resolve_all(&entrypoint, root, &cfg.dependencies, &mut registry).map_err(|errors| diag_err("utatuzi", errors))?;
        for res in program.resolved.values_mut() {
            filter_module_for_target(&mut res.module, &target).map_err(|errors| diag_err("sharti", errors))?;
        }
        filter_module_for_target(&mut program.merged_for_eval, &target).map_err(|errors| diag_err("sharti", errors))?;
        let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
        if !dup_errors.is_empty() {
            return Err(diag_err("semantiki", dup_errors));
        }
        let resolved_modules: HashSet<String> = program.resolved.keys().cloned().collect();
        for name in dependency_order(&program.resolved) {
            let res = &program.resolved[&name];
            let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
            semantic_check_with_env_and_modules(&res.module, false, ext_fns, ext_consts, resolved_modules.clone())
                .map_err(|errors| diag_err("semantiki", errors))?;
        }
        let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
        semantic_check_with_env_and_modules(&program.merged_for_eval, false, merged_fns, merged_consts, resolved_modules)
            .map_err(|errors| diag_err("semantiki", errors))?;

        for test_fn in discover_tests(&program.merged_for_eval) {
            names.push(test_fn.name);
        }
    }
    Ok(names)
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
        msg.push_str(&format!("- [{}:{}] {} @{}\n", d.stage, d.code, d.message, where_));
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-compile-{label}-{stamp}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    /// Cross-package `leta`: entrypoint imports a struct and a public constant from a path
    /// dependency and uses both. Exercises the resolver's path-dependency lookup and
    /// merge_for_eval's struct/constant handling end-to-end through compile_project.
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
        fs::write(pkg_src.join("greeter.as"), "umma kazi salamu() -> Neno { rejesha \"hi\" }\n")
            .expect("write vendored module");

        let mut deps = BTreeMap::new();
        deps.insert("greeter".to_string(), Dependency::Version("^1.0".to_string()));

        let found = find_module_file("greeter", &root, &deps);
        assert!(found.is_some(), "expected vendored greeter.as to resolve");
        let (path, is_asi) = found.unwrap();
        assert!(!is_asi);
        assert_eq!(path, pkg_src.join("greeter.as"));

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

        let compiled = compile_project(&root, None).expect("compile_project ok (target=wasm from manifest)");
        assert_eq!(compiled.target, Target("wasm".to_string()));
        assert!(
            !compiled.module.functions.iter().any(|f| f.name == "tu_native"),
            "tu_native should be filtered out of the in-memory module for target=wasm"
        );

        let artifact = emit_build_artifacts(&root, &compiled, None).expect("emit .asb");
        let bytes = fs::read(&artifact).expect("read .asb");
        let loaded = load_asb(&bytes).expect("load_asb");
        assert!(
            !loaded.functions.iter().any(|f| f.name == "tu_native"),
            "tu_native should be absent from the emitted .asb, not just the in-memory module"
        );
        assert!(loaded.functions.iter().any(|f| f.name == "kuu"));

        let _ = fs::remove_dir_all(&root);
    }
}
