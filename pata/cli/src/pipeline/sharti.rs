//! `#[sharti(...)]` conditional compilation: filter top-level items by active build target.

use asili_diagnostics::Diagnostic;
use asili_parser::{item_survives, Module, Target};

/// Remove top-level items (enums, structs, traits, impls, functions) whose `#[sharti(...)]`
/// predicate excludes `target`. Runs before semantic analysis, so excluded items never need to
/// type-check for a target that doesn't build them.
pub fn filter_module_for_target(module: &mut Module, target: &Target) -> Result<(), Vec<Diagnostic>> {
    let mut errors = Vec::new();

    module.enums.retain(|e| survives(&e.attrs, e.line, target, &mut errors));
    module.structs.retain(|s| survives(&s.attrs, s.line, target, &mut errors));
    module.traits.retain(|t| survives(&t.attrs, t.line, target, &mut errors));
    module.impls.retain(|i| survives(&i.attrs, i.line, target, &mut errors));
    module.functions.retain(|f| survives(&f.attrs, f.line, target, &mut errors));

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn survives(
    attrs: &[asili_parser::Attribute],
    line: usize,
    target: &Target,
    errors: &mut Vec<Diagnostic>,
) -> bool {
    match item_survives(attrs, target) {
        Ok(keep) => keep,
        Err(msg) => {
            errors.push(
                Diagnostic::new("SHA001", format!("sharti isiyoeleweka: {msg}"))
                    .with_stage("sharti")
                    .with_span(line, 1),
            );
            // Keep the item on error so a single bad predicate doesn't silently vanish code;
            // the collected diagnostic will fail the build.
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_lexer::tokenize;
    use asili_parser::parse_tokens;

    fn parse(src: &str) -> Module {
        let tokens = tokenize(src).expect("lex");
        parse_tokens(&tokens).expect("parse")
    }

    #[test]
    fn gated_function_filtered_by_target() {
        let src = r#"
#[sharti(lengo = "wasm")]
kazi tu_wasm() -> Tupu {
  rejesha
}
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  rejesha
}
"#;
        let mut module = parse(src);
        assert!(module.functions.iter().any(|f| f.name == "tu_wasm"));

        let mut native = module.clone();
        filter_module_for_target(&mut native, &Target::native()).expect("filter ok");
        assert!(!native.functions.iter().any(|f| f.name == "tu_wasm"));
        assert!(native.functions.iter().any(|f| f.name == "kuu"));

        filter_module_for_target(&mut module, &Target("wasm".into())).expect("filter ok");
        assert!(module.functions.iter().any(|f| f.name == "tu_wasm"));
    }

    #[test]
    fn gated_struct_filtered_by_target() {
        let src = r#"
#[sharti(lengo = "wasm")]
umbo TuWasm { x: Namba }
"#;
        let mut module = parse(src);
        filter_module_for_target(&mut module, &Target::native()).expect("filter ok");
        assert!(module.structs.is_empty());
    }

    #[test]
    fn unrecognized_predicate_key_is_an_error() {
        let src = r#"
#[sharti(bahati = "x")]
kazi tu() -> Tupu {
  rejesha
}
"#;
        let mut module = parse(src);
        let result = filter_module_for_target(&mut module, &Target::native());
        assert!(result.is_err());
    }
}
