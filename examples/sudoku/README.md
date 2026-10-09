# Sudoku

This example solves the Arto Inkala "world's hardest" Sudoku with an iterative MRV (fewest
candidates first) backtracking solver, and doubles as Asili's performance benchmark.

It demonstrates the concise syntax used by the solver:

- `kwa ... kutoka ... hadi` counter loops
- indexed list assignment, `orodha_rudia`, and compound assignment on list elements
  (`safu[r] |= x`)
- symbolic bitwise operators (`|`, `&`, `^`, `<<`), which bind tighter than comparisons
  (`tumika & x == 0`)
- grouped declarations and floor division (`weka r = i // N, c = i % N`)
- plain indexing without `?` (`b[p] == 0`)

Run it from this directory:

```bash
cargo run --release -p pata-cli -- jenga --tenda
```

The solver prints the board, the candidate-attempt count (90,665) and the backtrack count
(10,041).

## Benchmark

`bench/` holds the same algorithm in C, Rust and Python. `bench/run.sh [runs]` builds
everything and prints the best wall time per implementation, after checking that each one
reports 90,665 attempts:

```bash
./bench/run.sh 10
```

`pata jenga` compiles the bytecode ahead of time to native machine code with Asili's own
backend (`asili-nguvu`, no C compiler involved), and the standalone runner runs it.
Typical results on one core (whole process, including ~3.3 ms of process start-up):

| Implementation | Time |
|---|---|
| C (clang -O2) | 6.5 ms |
| C (gcc -O2) | 8.1 ms |
| Rust (-O) | 6.8 ms |
| Asili, native (`nguvu`) | 7.2 ms |
| Python 3 | 302 ms |

Solve only (minus an empty program's time): Asili ≈ 2.6 ms, clang C ≈ 3.1 ms, gcc C ≈ 4.7 ms.
