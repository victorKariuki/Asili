---
name: performance-guardrails
description: Keep Asili at C speed and keep its execution engines agreeing. Use before finishing ANY change to core/evaluator (bytecode.rs, aot.rs, native.rs, eval/ops.rs, eval/methods.rs, builtins/*), to parser lowering/desugaring (parse.rs), to the .asb format, or to examples/sudoku — and whenever adding an operator, builtin, method, opcode, Expr/Stmt variant or keyword. Covers the architecture invariants, the required tests, the Sudoku benchmark and its thresholds, and how to debug a regression.
---

# Performance guardrails

Asili runs the Arto Inkala Sudoku (`examples/sudoku`, 90,665 attempts) at C speed through
ahead-of-time native code. That result rests on a few invariants that are easy to break
silently: a change can keep every functional test green while making the hot loop 10× slower,
or while making the native tier disagree with the interpreter on one edge case. This skill is
the checklist that stops that. `docs/design/performance.md` has the full design and history.

## The execution tiers (one semantics, three engines)

| Tier | Where | When it runs |
|---|---|---|
| Tree-walking evaluator | `core/evaluator/src/eval/` | REPL, `pata jaribu`, AST `.asb` artifacts, any program the bytecode compiler can't lower |
| Typed register VM | `core/evaluator/src/bytecode.rs` | `.asb` bytecode artifacts when no native library is available (`ASILI_AOT=0`, no clang, wasm) |
| LLVM AOT native code | `core/evaluator/src/aot.rs` (+ `native.rs` analysis) | `pata jenga` emits `kilele/<name>.ll`, `clang -O2 -shared` builds `kilele/<name>.so`; `pata tenda`/`jenga --tenda`/runner load it if its hash matches the bytecode |

clang is needed only where `pata jenga` runs — not to build `pata`, not to run a built program.

## Invariants — never break these

1. **One implementation of every semantic rule.** Operators, casts, `?`/`jaribu`, truthiness,
   indexing, formatting, iteration and every method live in `eval/ops.rs` / `eval/methods.rs`;
   numeric opcode semantics live in `bytecode.rs::numeric_op` (used by both the interpreter loop
   and `exec_slow`). The VM and AOT call these; they never re-implement them. Generic-`Value`
   instructions in native code call back into `Vm::exec_slow` through the runtime ABI table.
   If you find yourself writing a second `match` over `BinaryOp` semantics, stop and call the
   shared function instead.
2. **One native backend.** LLVM IR text → clang. No JIT, no C transpiler. (A Cranelift JIT was
   removed on purpose; see commit `d77a8c4` if you need to read it.)
3. **Native code is bit-identical to the interpreter.** A `Namba` register may be lowered to
   `i64` only when `native::analyze_numbers` proves it whole, never NaN, never `-0.0`, and within
   ±2^53 — or when it is *speculated* with a ±2^53 guard that deoptimizes (`STATUS_DEOPT`) back
   to the interpreter at the failing pc. `sdiv`/`srem` only on proven integers. Bounds checks
   are dropped only for indices in `NumAnalysis::safe_index`. When in doubt, the analysis must
   return the conservative fact (TOP), never an optimistic one.
4. **Every opcode is described to the analysis.** A new `Opcode` needs entries in
   `native::num_reads`, `num_writes` and `list_writes`, and a `transfer` rule (or it falls to the
   conservative default — check that it does). Missing a write means the analysis believes a
   stale fact: silent miscompilation.
5. **Formats are versioned.** Changing `Opcode`'s serialized shape → bump `BYTECODE_VERSION`
   in `asb.rs`. Changing the runtime ABI struct, a native function signature, or anything the
   emitted IR assumes about the VM → bump `ABI_VERSION` in `aot.rs`. A stale `.so` must be
   rejected, never loaded.
6. **Unsupported means fallback, never crash.** When the bytecode compiler can't lower a
   construct it returns `None` and `pata jenga` emits the AST artifact
   (`compile_module_explained` says which `kazi`/line blocked it). The VM must never hit an
   "unsupported" error at run time for something it accepted at compile time.
7. **The hot path stays unboxed.** Numeric instructions touch only the `nums` (`f64`) and
   `lists` (`Vec<f64>`) register files — no `Value` construction, no allocation, no `HashMap`
   lookups per instruction. Fused instructions (`JumpIfNot` compare-and-branch, `ForStep`,
   `ListGet`) exist because they matter; don't split them back apart. Constants are preloaded
   registers.
8. **Terse source must stay fast source.** New sugar is lowered by the parser to forms the
   compiler already optimizes (`a // b` → `sakafu(a / b)` → `sdiv`; `a[i] op= v` →
   `a.ingiza(i, a[i] op v)` → `ListGet`/`ListSet`), so the concise spelling is never the slow one.

## Required tests for engine changes

- `cargo test -p asili-evaluator --test engines_agree` — each snippet on tree-walker, VM and
  AOT; values *and error messages* must match. Add a snippet for every new operator, method,
  builtin or syntax form.
- `cargo test -p asili-evaluator --test native_tiers` — interpreter vs AOT, bit-for-bit
  (NaN bit patterns excluded: LLVM and x86 disagree on NaN sign and Asili can't observe it).
  Add edge cases for anything numeric: `-0.0`, NaN, ±∞, ±2^53 and beyond (exercises deopt),
  negative `%`/`//`, shifts outside `0..=63`, out-of-range indices.
- `cargo test -p asili-evaluator --test bytecode` — compiler/VM unit behaviour.
- Tests that need AOT skip themselves without clang; run them where clang exists before
  pushing a change to `aot.rs`/`native.rs`.

## The benchmark — run it, don't guess

```bash
examples/sudoku/bench/run.sh 7      # best-of-7 whole-process wall time
```

It builds release `pata`, runs `pata jenga` on the example (which builds the native library),
and checks every implementation reports `Majaribio: 90665` before timing it.

Reference (2026-09, this container): C 7 ms · Rust 7 ms · **asili-aot 10 ms** · asili-vm
115 ms · Python 306 ms. About 3 ms of the Asili figure is `pata` start-up, not the solve
(solve-only: AOT ≈ 4.8 ms, gcc -O2 4.4 ms, clang -O2 3.2 ms).

**Thresholds** — treat any of these as a regression to fix before finishing:
- asili-aot more than ~1.6× the C time, or more than ~2 ms slower than its previous figure;
- asili-vm above ~130 ms;
- any "wrong result" (attempt count ≠ 90,665) — that is a correctness bug, not noise.

Timings on shared machines jitter by a millisecond or two; rerun before concluding. Record the
new figures in `docs/design/performance.md` when they change meaningfully.

## Debugging a regression

1. `ASILI_AOT=0` vs default: if only AOT slowed down, it's the IR or the analysis; if both did,
   it's the bytecode compiler or VM.
2. Did the program still lower to bytecode? `compile_module_explained` reports the first
   construct that forced the AST fallback (a 30× slowdown looks exactly like this).
3. Read `kilele/<name>.ll` (kept beside the `.so`): look for `call` into the runtime table
   (`exec_slow` spills) inside the hot loop, `sitofp`/`fptosi` pairs where a register should
   have stayed `i64`, and bounds-check branches that should be gone.
4. Instruction counts beat wall time for small deltas:
   `valgrind --tool=callgrind target/release/pata-cli tenda examples/sudoku/kilele/sudoku.asb`
   (AOT solve ≈ 30M instructions; clang C ≈ 27M).
5. `perf record`/`perf report` on the same command for where time goes.

## Checklists

**Adding an operator or syntax sugar:** parse to an existing AST form where possible
(`build_binary` in `parse.rs`); semantics in `eval/ops.rs`; if it needs an opcode, follow
invariants 4–5; a snippet in `engines_agree.rs` and edge cases in `native_tiers.rs`; update the
VS Code grammar and `asili_lexer::KEYWORDS` (the single keyword list LSP and formatter share).

**Adding a builtin:** implement once in `builtins/*.rs` (use `Value::sawa`/`Value::kosa` and
`arg_str`/`value::arg_f64`); declare its contract in `core/parser/src/builtins.rs` (the analyzer
*and* LSP completion read it); the VM reaches it through `BuiltinTable` automatically. If it is
hot and numeric, consider a dedicated opcode (as `sakafu`/`dari` have).

**Adding a method:** add it to `eval/methods.rs` only (pure, mutating or callback, plus the
matching `is_*` predicate); the evaluator, VM and AOT all pick it up from there.

**Adding an `Expr`/`Stmt` variant:** update `Expr::children` in `ast.rs` (linters, LSP and the
parser's own checks walk the tree through it); the bytecode compiler returns `None` for it until
lowered (invariant 6).

## Chains into

- `sync-pata-toolchain` when a builtin/keyword/type changed.
- `release-and-git-flow` for the changelog entry (performance numbers belong there too).
- `update-wiki` — the wiki's performance/architecture pages mirror `docs/design/performance.md`.
