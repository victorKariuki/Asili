# Standard library interface stubs

One `<module>.asi` per builtin module: its builtin `umbo`s, constants and functions written in
Asili, each with its description. The implementations are the builtins in
`core/evaluator/src/builtins/`.

These files are **generated** from the builtin table, `BUILTIN_MODULES` in
`core/parser/src/builtins.rs` — the same table the compiler checks calls against and the editor
(LSP) shows in hover, signature help and completion. Do not edit them: change the table, then

    ASILI_GOLDEN=write cargo test -p pata-core stdlib_stubs

`cargo test -p pata-core` fails while a stub differs from what the table renders.
