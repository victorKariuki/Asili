# evaluator stub

Purpose: evaluate/lower AST into execution form (`.asb` / VM input).

Planned API:
- `lower(ast: AstProgram) -> BytecodeModule`
- ownership-aware temporary/value tracking
- hooks for strict vs standard allocation behavior
