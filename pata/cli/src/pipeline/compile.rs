use crate::commands::CliError;
use crate::pipeline::interface_registry::InterfaceRegistry;
use crate::pipeline::project::{load_project_config, read_lockfile, ProjectConfig};
use crate::pipeline::resolve::{
    check_duplicate_imports, dependency_order, find_module_file, merge_for_semantic, resolve_all,
    ResolvedProgram,
};
use asili_diagnostics::Diagnostic;
use asili_evaluator::{emit_asb, load_asb, execute_tests, validate_module, TestResult};
use asili_lexer::tokenize;
use asili_parser::{discover_tests, parse_tokens, semantic_check_with_env, Function, Module};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Content-addressable cache key from name and source (e.g. for single-file builds).
pub fn cache_key(name: &str, source: &str) -> String {
    let mut h = DefaultHasher::new();
    name.hash(&mut h);
    source.hash(&mut h);
    format!("{:016x}", h.finish())
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
}

/// Compute content hash of all inputs to a project build (entry + resolved modules).
pub fn project_input_hash(
    root: &Path,
    entry_path: &Path,
    entry_content: &str,
    program: &ResolvedProgram,
) -> String {
    let mut h = DefaultHasher::new();
    entry_path.display().to_string().hash(&mut h);
    entry_content.hash(&mut h);
    let mut pairs: Vec<(String, String)> = Vec::new();
    for name in dependency_order(&program.resolved) {
        let content = if let Some((path, _)) = find_module_file(name.as_str(), root) {
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

pub fn compile_project(root: &Path) -> Result<CompileOutput, CliError> {
    let mut cfg = load_project_config(root)?;
    if let Some(locked) = read_lockfile(root)? {
        cfg.dependencies = locked;
    }
    let source = fs::read_to_string(&cfg.entrypoint).map_err(|e| {
        CliError::new(
            format!("imeshindwa kusoma chanzo {}: {e}", cfg.entrypoint.display()),
            1,
        )
    })?;

    let tokens = tokenize(&source).map_err(|errors| diag_err("lex", errors))?;
    let entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("parse", errors))?;
    let mut registry = InterfaceRegistry::new(root.to_path_buf());
    registry.register_builtins();
    registry.load_stdlib()?;
    let prelude = registry.prelude_env();
    let program = resolve_all(&entrypoint, root, &mut registry).map_err(|errors| diag_err("resolve", errors))?;

    let input_hash = project_input_hash(root, &cfg.entrypoint, &source, &program);
    let target_dir = root.join("target");
    let manifest_path = target_dir.join(format!("{}.build.manifest", cfg.name));
    let asb_path = target_dir.join(format!("{}.asb", cfg.name));
    if manifest_path.is_file() && asb_path.is_file() {
        if let Ok(manifest_content) = fs::read_to_string(&manifest_path) {
            let stored = manifest_content
                .lines()
                .find(|l| l.starts_with("input_hash="))
                .and_then(|l| l.strip_prefix("input_hash=").map(str::trim));
            if stored == Some(input_hash.as_str()) {
                let bytes = fs::read(&asb_path).map_err(|e| {
                    CliError::new(format!("imeshindwa kusoma cache {}: {e}", asb_path.display()), 1)
                })?;
                let module = load_asb(&bytes).map_err(|e| {
                    CliError::new(format!("kuipakua asb: {e}"), 1)
                })?;
                return Ok(CompileOutput {
                    module,
                    source,
                    source_path: cfg.entrypoint.clone(),
                    config: cfg,
                    input_hash: Some(input_hash),
                    from_cache: true,
                });
            }
        }
    }

    let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
    if !dup_errors.is_empty() {
        return Err(diag_err("semantic", dup_errors));
    }

    for name in dependency_order(&program.resolved) {
        let res = &program.resolved[&name];
        let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
        semantic_check_with_env(&res.module, false, ext_fns, ext_consts)
            .map_err(|errors| diag_err("semantic", errors))?;
    }

    let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
    semantic_check_with_env(&entrypoint, true, merged_fns, merged_consts)
        .map_err(|errors| diag_err("semantic", errors))?;
    validate_module(&program.merged_for_eval).map_err(|errors| diag_err("evaluator", errors))?;

    Ok(CompileOutput {
        module: program.merged_for_eval,
        source,
        source_path: cfg.entrypoint.clone(),
        config: cfg,
        input_hash: Some(input_hash),
        from_cache: false,
    })
}

/// Compile a single .as file without pata.toml. Root for resolve/stdlib is the file's parent.
pub fn compile_single_file(entry_path: &Path) -> Result<CompileOutput, CliError> {
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
    };

    let tokens = tokenize(&source).map_err(|errors| diag_err("lex", errors))?;
    let entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("parse", errors))?;
    let mut registry = InterfaceRegistry::new(root.clone());
    registry.register_builtins();
    registry.load_stdlib()?;
    let prelude = registry.prelude_env();
    let program = resolve_all(&entrypoint, &root, &mut registry).map_err(|errors| diag_err("resolve", errors))?;

    let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
    if !dup_errors.is_empty() {
        return Err(diag_err("semantic", dup_errors));
    }

    for dep_name in dependency_order(&program.resolved) {
        let res = &program.resolved[&dep_name];
        let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
        semantic_check_with_env(&res.module, false, ext_fns, ext_consts)
            .map_err(|errors| diag_err("semantic", errors))?;
    }

    let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
    semantic_check_with_env(&entrypoint, true, merged_fns, merged_consts)
        .map_err(|errors| diag_err("semantic", errors))?;
    validate_module(&program.merged_for_eval).map_err(|errors| diag_err("evaluator", errors))?;

    Ok(CompileOutput {
        module: program.merged_for_eval,
        source,
        source_path: config.entrypoint.clone(),
        config,
        input_hash: None,
        from_cache: false,
    })
}

pub fn emit_build_artifacts(root: &Path, compiled: &CompileOutput, out_dir: Option<&Path>) -> Result<PathBuf, CliError> {
    let target = out_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.join("target"));
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
        .map(|h| format!("input_hash={}\n", h))
        .unwrap_or_default();
    let manifest = format!(
        "project={}\nentry={}\nfunctions={}\nartifact={}.asb\n{}",
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
        let key = cache_key(&compiled.config.name, &compiled.source);
        let cache_artifact = cache_dir.join(format!("{key}.asb"));
        let _ = fs::write(&cache_artifact, &asb);
    }

    Ok(artifact)
}

pub fn run_project_tests(root: &Path, filter: Option<&str>, fail_fast: bool) -> Result<Vec<TestResult>, CliError> {
    let cfg = load_project_config(root)?;
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
        let tokens = tokenize(&source).map_err(|errors| diag_err("lex", errors))?;
        let entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("parse", errors))?;

        let program = resolve_all(&entrypoint, root, &mut registry).map_err(|errors| diag_err("resolve", errors))?;
        let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
        if !dup_errors.is_empty() {
            return Err(diag_err("semantic", dup_errors));
        }
        for name in dependency_order(&program.resolved) {
            let res = &program.resolved[&name];
            let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
            semantic_check_with_env(&res.module, false, ext_fns, ext_consts)
                .map_err(|errors| diag_err("semantic", errors))?;
        }
        let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
        semantic_check_with_env(&entrypoint, false, merged_fns, merged_consts)
            .map_err(|errors| diag_err("semantic", errors))?;

        for test_fn in discover_tests(&program.merged_for_eval) {
            modules_and_tests.push((program.merged_for_eval.clone(), test_fn));
        }
    }

    if let Some(f) = filter {
        modules_and_tests.retain(|(_, t)| t.name.contains(f));
    }
    Ok(execute_tests(&modules_and_tests, fail_fast))
}

/// List test names discovered in the project (no execution). For use with `pata jaribu --list`.
pub fn list_project_tests(root: &Path) -> Result<Vec<String>, CliError> {
    let cfg = load_project_config(root)?;
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
        let tokens = tokenize(&source).map_err(|errors| diag_err("lex", errors))?;
        let entrypoint = parse_tokens(&tokens).map_err(|errors| diag_err("parse", errors))?;

        let program = resolve_all(&entrypoint, root, &mut registry).map_err(|errors| diag_err("resolve", errors))?;
        let dup_errors = check_duplicate_imports(&entrypoint, &program.resolved, &prelude);
        if !dup_errors.is_empty() {
            return Err(diag_err("semantic", dup_errors));
        }
        for name in dependency_order(&program.resolved) {
            let res = &program.resolved[&name];
            let (ext_fns, ext_consts) = merge_for_semantic(&res.module, &program.resolved, &prelude);
            semantic_check_with_env(&res.module, false, ext_fns, ext_consts)
                .map_err(|errors| diag_err("semantic", errors))?;
        }
        let (merged_fns, merged_consts) = merge_for_semantic(&entrypoint, &program.resolved, &prelude);
        semantic_check_with_env(&entrypoint, false, merged_fns, merged_consts)
            .map_err(|errors| diag_err("semantic", errors))?;

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
        let entry = entry.map_err(|e| CliError::new(format!("hitilafu ya entry: {e}"), 1))?;
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
