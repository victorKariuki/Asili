# evaluator

Purpose: lower Asili modules to `.asb` bytecode and run them as native code from the in-house
`nguvu` backend (machine code on x86-64/AArch64, a wasm module in the browser). There is no
interpreter and no fallback: a construct the bytecode compiler can't lower is a build error.

Entry points: `compile_module_explained` (bytecode, or the `kazi` and line that blocked it),
`NativeProgram` (build once, call functions on fresh hosts; `run_function`/`run_main` wrap it),
`run_artifact` (an `.asb` with its `.nguvu` image), `ReplSession`, `run_main_with_debug_hook`,
and the test runner (`execute_tests_with_timeout`, `run_test_with_coverage`).
