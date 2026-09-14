use super::{CliError, CliResult};
use crate::pipeline::compile::{
    cache_key, compile_project, compile_single_file, emit_build_artifacts,
};
use asili_evaluator::{load_asb, run_main};
use std::fs;
use std::path::{Path, PathBuf};

const JENGA_USAGE: &str = r#"matumizi: pata jenga [faili.as] [chagua...] [hoja za kuu...]

Jenga mradi kutoka pata.toml au faili moja (bila mradi).

Chagua:
  --tenda          Baada ya kujenga, tenda kazi kuu na hoja zinazofuata.
  --pato <njia>    Mahali pa kuweka kilele (default: kilele/).
  --namna <dev|release|embedded>  Namna ya kujenga (haijatumika bado).
  --lengo <lengo>  Lengo la kujenga (mf. "native", "wasm"). Hupita
                   [jenga] lengo katika pata.toml; default "native".
  --msaada         Onyesha ujumbe huu.

Hoja za kuu: Kila neno lisilokuwa chagua linapewa kwa kuu(hoja: Orodha<Neno>).

Mifano:
  pata jenga
  pata jenga --tenda
  pata jenga --tenda foo bar
  pata jenga script.as --tenda
"#;

// Contract: ../../commands/jenga.md
pub fn run(args: &[String]) -> CliResult {
    if args
        .iter()
        .any(|a| a == "--msaada")
    {
        print!("{JENGA_USAGE}");
        return Ok(());
    }
    let (_profile, out, do_run, single_file, program_args, build_target) = parse_args(args)?;
    if let Some(ref path) = single_file {
        if !path.exists() {
            return Err(CliError::new(
                format!("faili haipo: {}", path.display()),
                1,
            ));
        }
        if !path.is_file() {
            return Err(CliError::new(
                format!("si faili: {} (inahitaji faili .as)", path.display()),
                1,
            ));
        }
    }

    let (root, compiled) = if let Some(ref path) = single_file {
        let root = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let target = out
            .clone()
            .unwrap_or_else(|| root.join("kilele"));
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("script");
        let source = fs::read_to_string(path).map_err(|e| {
            CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1)
        })?;
        let key = cache_key(name, &source, build_target.as_deref().unwrap_or("native"));
        let cache_path = target.join(".asb-cache").join(format!("{key}.asb"));
        if cache_path.exists() {
            fs::create_dir_all(&target).map_err(|e| {
                CliError::new(format!("imeshindwa kuunda {}: {e}", target.display()), 1)
            })?;
            let asb_bytes = fs::read(&cache_path).map_err(|e| {
                CliError::new(format!("imeshindwa kusoma cache {}: {e}", cache_path.display()), 1)
            })?;
            let module = load_asb(&asb_bytes).map_err(|e| {
                CliError::new(format!("kuipakia asb: {e}"), 1)
            })?;
            let artifact = target.join(format!("{name}.asb"));
            fs::write(&artifact, &asb_bytes).map_err(|e| {
                CliError::new(format!("imeshindwa kuandika {}: {e}", artifact.display()), 1)
            })?;
            let meta = target.join(format!("{name}.build.manifest"));
            let manifest = format!(
                "mradi={name}\nkuingia={}\nkazi={}\nkilele={name}.asb\n",
                path.display(),
                module.functions.len()
            );
            let _ = fs::write(&meta, manifest);
            println!("imejengwa (cache): {}", artifact.display());
            if do_run {
                run_main(&module, program_args).map_err(|e| {
                    CliError::new(format!("kuendesha kuu: {e}"), 1)
                })?;
            }
            return Ok(());
        }
        let compiled = compile_single_file(path, build_target.as_deref())?;
        (root, compiled)
    } else {
        let root = Path::new(".");
        let compiled = compile_project(root, build_target.as_deref())?;
        (root.to_path_buf(), compiled)
    };

    let _artifact = if compiled.from_cache {
        let target = out
            .as_ref()
            .cloned()
            .unwrap_or_else(|| root.join("kilele"));
        let a = target.join(format!("{}.asb", compiled.config.name));
        println!("imejengwa (cache): {}", a.display());
        a
    } else {
        let a = emit_build_artifacts(&root, &compiled, out.as_deref())?;
        println!("imejengwa: {}", a.display());
        a
    };
    if do_run {
        run_main(&compiled.module, program_args).map_err(|e| {
            CliError::new(format!("kuendesha kuu: {e}"), 1)
        })?;
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
pub fn parse_args(
    args: &[String],
) -> Result<(String, Option<PathBuf>, bool, Option<PathBuf>, Vec<String>, Option<String>), CliError> {

    let mut profile = String::from("dev");
    let mut out = None;
    let mut do_run = false;
    let mut single_file = None;
    let mut program_args = Vec::new();
    let mut target = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--namna" => {
                let Some(p) = args.get(i + 1) else {
                    return Err(CliError::new("--namna inahitaji thamani", 2));
                };
                profile = p.clone();
                i += 2;
            }
            "--pato" => {
                let Some(p) = args.get(i + 1) else {
                    return Err(CliError::new("--pato inahitaji njia", 2));
                };
                out = Some(PathBuf::from(p));
                i += 2;
            }
            "--lengo" => {
                let Some(t) = args.get(i + 1) else {
                    return Err(CliError::new("--lengo inahitaji thamani", 2));
                };
                target = Some(t.clone());
                i += 2;
            }
            "--tenda" => {
                do_run = true;
                i += 1;
            }
            other => {
                if other.ends_with(".as") && single_file.is_none() {
                    single_file = Some(PathBuf::from(other));
                } else {
                    program_args.push(other.to_string());
                }
                i += 1;
            }
        }
    }
    Ok((profile, out, do_run, single_file, program_args, target))
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::commands::TEST_CWD_LOCK;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn emits_asb_artifact() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");
        run(&[]).expect("jenga ok");
        let asb_bytes = fs::read("kilele/app.asb").expect("artifact");
        assert!(
            asb_bytes.starts_with(b"ASB-STUB"),
            "asb should have ASB-STUB header"
        );
        let module = asili_evaluator::load_asb(&asb_bytes).expect("load_asb");
        assert!(!module.functions.is_empty(), "asb should contain merged module");
        let manifest = fs::read_to_string("kilele/app.build.manifest").expect("per-artifact manifest");
        assert!(manifest.contains("mradi=app"), "manifest should have mradi=app");
        assert!(manifest.contains("kuingia="));
        assert!(manifest.contains("kazi="));
        assert!(manifest.contains("kilele=app.asb"), "manifest should point at .asb");
        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn two_builds_produce_two_manifests_no_overwrite() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");
        run(&[]).expect("jenga project ok");
        fs::write(
            root.join("other.as"),
            "leta mfumo\nleta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"y\") }",
        )
        .expect("write other.as");
        run(&["other.as".into()]).expect("jenga single file ok");
        let app_manifest = fs::read_to_string("kilele/app.build.manifest").expect("app manifest");
        let other_manifest = fs::read_to_string("kilele/other.build.manifest").expect("other manifest");
        assert!(app_manifest.contains("mradi=app") && app_manifest.contains("kilele=app.asb"));
        assert!(other_manifest.contains("mradi=other") && other_manifest.contains("kilele=other.asb"));
        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    fn temp_project() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-jenga-{stamp}"));
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::create_dir_all(dir.join("lib/std")).expect("mkdir lib/std");
        fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        )
        .expect("write manifest");
        fs::write(
            dir.join("src/kuu.as"),
            "leta mfumo\nleta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }",
        )
        .expect("src");
        fs::write(
            dir.join("lib/std/mfumo.asi"),
            "kazi chapisha(ujumbe: Neno) -> Tupu\n",
        )
        .expect("stdlib mfumo");
        dir
    }

    #[test]
    fn jenga_run_executes_kuu() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        std::env::set_current_dir(&root).expect("chdir");
        run(&["--tenda".into()]).expect("jenga --tenda ok");
        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn jenga_resolves_builtin_hisabati_without_file() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        fs::write(
            root.join("src/kuu.as"),
            "leta hisabati\nleta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { weka x = jumla(2, 3) chapisha(\"ok\") }",
        )
        .expect("write");
        std::env::set_current_dir(&root).expect("chdir");
        run(&["jenga".into()]).expect("jenga with leta hisabati (builtin, no lib/std/hisabati.asi)");
        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn jenga_core_complete_struct_impl_index_kamusi() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let root = temp_project();
        let src = r#"leta mfumo
leta matumizi
umbo Pika { x: Namba }
shughuli ya Pika { kazi ongeza(self: Pika, d: Namba) -> Namba { rejesha self.x + d } }
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka p = Pika { x: 5 }
  weka n = p.ongeza(3)
  chapisha("ok")
}"#;
        fs::write(root.join("src/kuu.as"), src).expect("write");
        std::env::set_current_dir(&root).expect("chdir");
        run(&["--tenda".into()]).expect("jenga --tenda with struct and impl");
        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn jenga_single_file_tenda() {
        let _guard = TEST_CWD_LOCK.lock().expect("lock");
        let original = std::env::current_dir().expect("cwd");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-jenga-single-{stamp}"));
        fs::create_dir_all(&dir).expect("mkdir");
        let script = dir.join("one.as");
        fs::write(
            &script,
            "leta mfumo\nleta matumizi\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }",
        )
        .expect("write");
        std::env::set_current_dir(&dir).expect("chdir");
        let path_str = script.to_string_lossy().to_string();
        run(&[path_str, "--tenda".into()]).expect("jenga single file --tenda ok");
        std::env::set_current_dir(&original).expect("restore cwd");
        let _ = fs::remove_dir_all(&dir);
    }
}
