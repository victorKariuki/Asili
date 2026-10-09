# Standard library interface stubs

One `<module>.asi` per builtin module (`core/parser/src/builtins.rs::BUILTIN_MODULE_NAMES`): the
signatures of its functions and constants, written in Asili. The implementations are the
builtins in `core/evaluator/src/builtins/`; these stubs describe them to tools and readers.

The stubs are hand-written, so `pata-core`'s `stdlib_stubs_match_builtin_export_tables` test
checks each one against its module's export table (missing or extra names, arity, types) and
prints the line to add or change. Run `cargo test -p pata-core` after changing a builtin.
