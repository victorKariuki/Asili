//! Logic error lint rules

use asili_diagnostics::Diagnostic;
use asili_parser::Module;

/// Check for logic errors (placeholder for future depth)
/// Currently a no-op; real unreachable-code detection requires deeper AST inspection
pub fn check_logic_errors(_module: &Module) -> Vec<Diagnostic> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_lexer::tokenize;
    use asili_parser::parse_tokens;

    fn lint(src: &str) -> Vec<Diagnostic> {
        let tokens = tokenize(src).expect("tokenize");
        let module = parse_tokens(&tokens).expect("parse");
        check_logic_errors(&module)
    }

    #[test]
    fn logic_checks_placeholder() {
        let src = "kazi test() -> Tupu { rejesha Tupu }";
        let diags = lint(src);
        assert_eq!(diags.len(), 0);
    }
}
