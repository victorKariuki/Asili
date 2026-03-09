use asili_diagnostics::Diagnostic;
use asili_lexer::Token;
use std::collections::HashMap;

mod ast;
mod cursor;
mod parse;
mod semantic;
pub use ast::*;
pub use semantic::parse_value_type;

use cursor::Parser;

pub fn parse_tokens(tokens: &[Token]) -> Result<Module, Vec<Diagnostic>> {
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module();
    if parser.errors.is_empty() {
        Ok(module)
    } else {
        Err(parser.errors)
    }
}

pub fn discover_tests(module: &Module) -> Vec<Function> {
    module.functions.iter().filter(|f| f.is_test).cloned().collect()
}

pub fn semantic_check(module: &Module) -> Result<(), Vec<Diagnostic>> {
    semantic_check_with_options(module, true)
}

pub fn semantic_check_with_options(module: &Module, require_main: bool) -> Result<(), Vec<Diagnostic>> {
    semantic_check_with_env(module, require_main, HashMap::new(), HashMap::new())
}

pub fn semantic_check_with_env(
    module: &Module,
    require_main: bool,
    extern_functions: HashMap<String, FnContract>,
    extern_constants: HashMap<String, ValueType>,
) -> Result<(), Vec<Diagnostic>> {
    let errors = semantic::run_semantic_check(module, require_main, extern_functions, extern_constants);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
