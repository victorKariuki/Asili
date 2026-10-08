//! A program built to native code once and called many times, each call on a fresh host (so
//! module constants are computed afresh, as each run of a program computes them): how
//! `pata jaribu` runs every test and fixture of a module.

use std::sync::Arc;

use asili_parser::Module;

use crate::bytecode::BytecodeProgram;
use crate::value::{EvalError, Value};

/// A module's bytecode and the native code built from it.
#[derive(Clone)]
pub struct NativeProgram {
    program: Arc<BytecodeProgram>,
    native: Arc<crate::aot::NativeLibrary>,
}

impl NativeProgram {
    /// Lower `module` to bytecode and build its native code; the error names the `kazi` and
    /// line that could not be lowered, or why no native code could be built here.
    pub fn build(module: &Module) -> Result<Self, EvalError> {
        Self::build_with(module, crate::bytecode::CompileOptions::default())
    }

    /// [`NativeProgram::build`] with `options` (`lines` for [`NativeProgram::call_with_coverage`]).
    pub fn build_with(
        module: &Module,
        options: crate::bytecode::CompileOptions,
    ) -> Result<Self, EvalError> {
        let program = crate::bytecode::compile_module_with(module, options).map_err(|why| {
            EvalError::Unknown(format!("{why} bado haiwezi kugeuzwa kuwa msimbo asilia"))
        })?;
        Self::from_program(program)
    }

    /// Build native code for an already lowered program.
    pub fn from_program(program: BytecodeProgram) -> Result<Self, EvalError> {
        if !crate::nguvu::supported() {
            return Err(EvalError::Unknown(
                "jukwaa hili halina msimbo asilia (nguvu)".into(),
            ));
        }
        let native = crate::nguvu::compile(&program)
            .map_err(|e| EvalError::Unknown(format!("msimbo asilia haukujengwa: {e}")))?;
        Ok(NativeProgram {
            program: Arc::new(program),
            native: Arc::new(native),
        })
    }

    /// Call the program's `kazi` (or `Umbo::njia` method) called `name` on a fresh host.
    pub fn call(&self, name: &str, args: Vec<Value>) -> Result<Value, EvalError> {
        self.host().call_by_name(name, args)
    }

    /// Like [`NativeProgram::call`], also returning the source lines that ran on this thread
    /// (the program must be built with `CompileOptions::lines`); a call that fails still reports
    /// the lines it ran.
    pub fn call_with_coverage(
        &self,
        name: &str,
        args: Vec<Value>,
    ) -> (Result<Value, EvalError>, std::collections::HashSet<usize>) {
        let mut host = self.host();
        host.coverage = Some(Default::default());
        let result = host.call_by_name(name, args);
        (result, host.coverage.take().unwrap_or_default())
    }

    /// Like [`NativeProgram::call`], with the debugger `hook` told of every statement (the
    /// program must be built with `CompileOptions { lines: true, bindings: true }`).
    pub fn call_with_debugger(
        &self,
        name: &str,
        args: Vec<Value>,
        hook: std::sync::Arc<dyn crate::debug_hook::DebugHook>,
    ) -> Result<Value, EvalError> {
        let mut host = self.host();
        host.debug = Some(hook);
        host.call_by_name(name, args)
    }

    fn host(&self) -> crate::host::Host<'_> {
        let shared = crate::spawn::Shared::Code {
            program: self.program.clone(),
            native: self.native.clone(),
        };
        crate::host::Host::new(&self.program, &self.native, Some(shared))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::NativeProgram;
    use crate::value::Value;

    /// A signal that arrives while native code runs dispatches to its `sikiliza_ishara` handler
    /// the next time native code calls into the host, as the tree-walker dispatches before its
    /// next statement.
    #[test]
    fn pending_signals_run_their_handler() {
        if !crate::nguvu::supported() {
            return;
        }
        let path = std::env::temp_dir().join(format!("asili-ishara-{}", std::process::id()));
        let source = format!(
            r#"
            kazi mshikaji() -> Tupu {{
                weka r = andika_faili("{p}", "imepokelewa")
            }}
            kazi kuu() -> Neno {{
                weka n = "a" + "b"
                rejesha soma_faili("{p}").angu("hakuna")
            }}
            "#,
            p = path.display()
        );
        let tokens = asili_lexer::tokenize(&source).expect("tokenize");
        let module = asili_parser::parse_tokens(&tokens).expect("parse");
        let program = NativeProgram::build(&module).expect("native");
        // SIGUSR2: nothing else in the test binary uses it.
        crate::signal::register_handler(12, "mshikaji".to_string());
        crate::signal::set_pending(12);
        let got = program.call("kuu", vec![]);
        crate::signal::clear_handler(12);
        let _ = std::fs::remove_file(&path);
        assert_eq!(got.expect("kuu"), Value::neno("imepokelewa"));
    }
}
