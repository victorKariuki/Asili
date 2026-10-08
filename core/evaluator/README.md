# evaluator

Purpose: evaluate Asili modules, and lower them to `.asb` bytecode that the in-house `nguvu`
backend compiles to native code (there is no bytecode interpreter). Programs using unsupported
constructs retain the serialized-AST evaluator fallback; where there is no native backend the
tree-walker runs the syntax tree a bytecode artifact carries.

The public `compile_module`/`run_bytecode_function` API covers arithmetic, comparisons, loops,
lists, indexing, mutation, casts, builtin calls, and user-function calls. `pata jenga` selects
this native-code path while preserving the tree-walk evaluator for the rest of the language.
