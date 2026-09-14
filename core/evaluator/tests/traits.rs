//! Sifa (traits): trait-impl parsing (`kwa` and `:` syntax), method dispatch, and the
//! method-signature completeness checker (SEM105).

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, extern_env_from_imports, Module};

fn compile(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    semantic_check_with_env(&module, false, fns, consts).expect("semantic check");
    module
}

#[test]
fn kwa_syntax_trait_impl_method_dispatches() {
    // Regression test: `shughuli ya Target kwa Trait { ... }` used to parse with target/trait
    // swapped, so `imp.target` never matched the struct's real name anywhere method dispatch
    // looks it up (core/parser/src/parse.rs's parse_impl_decl, core/evaluator/src/eval/expr.rs's
    // struct-method search). Fixed; this asserts the fix, not just that it compiles.
    let src = r#"
        umbo Paka { jina: Neno }

        sifa Inayoonyeshwa {
            kazi onyesha(self: Self) -> Neno
        }

        shughuli ya Paka kwa Inayoonyeshwa {
            kazi onyesha(self: Paka) -> Neno {
                rejesha "Paka(" + self.jina + ")"
            }
        }

        kazi jaribu() -> Neno {
            weka pk = Paka { jina: "Whiskers" }
            rejesha pk.onyesha()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("Paka(Whiskers)".to_string()));
}

#[test]
fn colon_syntax_trait_impl_method_dispatches() {
    let src = r#"
        umbo Paka { jina: Neno }

        sifa Inayoonyeshwa {
            kazi onyesha(self: Self) -> Neno
        }

        shughuli ya Paka: Inayoonyeshwa {
            kazi onyesha(self: Paka) -> Neno {
                rejesha "Paka(" + self.jina + ")"
            }
        }

        kazi jaribu() -> Neno {
            weka pk = Paka { jina: "Whiskers" }
            rejesha pk.onyesha()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("Paka(Whiskers)".to_string()));
}

#[test]
fn incomplete_impl_is_rejected_sem105() {
    let src = r#"
        umbo Paka { jina: Neno }

        sifa Inayoonyeshwa {
            kazi onyesha(self: Self) -> Neno
            kazi jina_fupi(self: Self) -> Neno
        }

        shughuli ya Paka kwa Inayoonyeshwa {
            kazi onyesha(self: Paka) -> Neno {
                rejesha "Paka(" + self.jina + ")"
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    let errs = result.expect_err("missing jina_fupi should be a semantic error");
    assert!(
        errs.iter().any(|d| d.code == "SEM105"),
        "expected SEM105 for the unimplemented trait method, got: {errs:?}"
    );
}

#[test]
fn impl_with_mismatched_return_type_is_rejected_sem105() {
    let src = r#"
        umbo Paka { jina: Neno }

        sifa Inayoonyeshwa {
            kazi onyesha(self: Self) -> Neno
        }

        shughuli ya Paka kwa Inayoonyeshwa {
            kazi onyesha(self: Paka) -> Namba {
                rejesha 1
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    let errs = result.expect_err("wrong return type should not satisfy the trait");
    assert!(errs.iter().any(|d| d.code == "SEM105"));
}

#[test]
fn empty_trait_body_has_no_completeness_requirements() {
    let src = r#"
        umbo Paka { jina: Neno }

        sifa Marker { }

        shughuli ya Paka kwa Marker { }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
    "#;
    let module = compile(src);
    let _ = module;
}

#[test]
fn plain_impl_without_trait_still_works() {
    let src = r#"
        umbo Paka { jina: Neno }

        shughuli ya Paka {
            kazi onyesha(self: Paka) -> Neno {
                rejesha "Paka(" + self.jina + ")"
            }
        }

        kazi jaribu() -> Neno {
            weka pk = Paka { jina: "Whiskers" }
            rejesha pk.onyesha()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("Paka(Whiskers)".to_string()));
}
