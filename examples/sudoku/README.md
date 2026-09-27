# Sudoku

This example solves the Arto Inkala Sudoku fixture with an iterative MRV
backtracking solver.

It demonstrates the concise syntax used by the solver:

- `kwa ... kutoka ... hadi` counter loops
- indexed list assignment and `orodha_rudia`
- symbolic bitwise operators (`|`, `&`, `^`, `<<`)
- compound assignment
- value-producing `ikiwa` expressions

Run it from this directory:

```bash
cargo run -p pata-cli -- jenga --tenda
```

The solver prints the board, candidate-attempt count, and backtrack count.
