# parser stub

Purpose: parse lexer tokens into AST nodes.

Planned API:
- `parse_program(tokens: &[Token]) -> AstProgram`
- expression parsing with operator precedence (`kama` tighter than arithmetic)
- recoverable parse errors with context for diagnostics
