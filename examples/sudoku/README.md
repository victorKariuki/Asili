# Sudoku

This example solves the Arto Inkala "world's hardest" Sudoku with an iterative MRV (fewest
candidates first) backtracking solver, and doubles as Asili's performance benchmark.

It demonstrates the concise syntax used by the solver:

- `kwa ... kutoka ... hadi` counter loops
- indexed list assignment, `orodha_rudia`, and compound assignment on list elements
  (`safu[r] |= x`)
- symbolic bitwise operators (`|`, `&`, `^`, `<<`), which bind tighter than comparisons
  (`tumika & x == 0`)
- grouped declarations (`weka r = sakafu(i / N), c = i % N`)

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

`pata jenga` compiles the bytecode ahead of time to native code through LLVM when `clang` is
installed (`asili-aot`); otherwise `pata tenda` JIT-compiles it with Cranelift (`asili-jit`,
or `ASILI_AOT=0`), and `ASILI_JIT=0` as well runs the register-VM interpreter (`asili-vm`).
Typical results on one core (whole process, including startup):

| Implementation | Time |
|---|---|
| C (gcc -O2) | 8 ms |
| Rust (-O) | 7 ms |
| Asili, AOT (LLVM) | 11 ms |
| Asili, JIT (Cranelift) | 40 ms |
| Asili, VM interpreter | 110 ms |
| Python 3.11 | 305 ms |
