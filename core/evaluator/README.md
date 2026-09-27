# evaluator

Purpose: evaluate Asili modules and lower the loop/list-heavy Sudoku subset into executable `.asb`
stack bytecode. Programs using unsupported constructs retain the serialized-AST evaluator
fallback.

The public `compile_module`/`run_bytecode_function` API covers arithmetic, comparisons, loops,
lists, indexing, mutation, casts, builtin calls, and user-function calls. `pata jenga` selects
this VM artifact path while preserving the tree-walk evaluator for the rest of the language.
