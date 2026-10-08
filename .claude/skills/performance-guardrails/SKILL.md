---
name: performance-guardrails
description: Keep Asili at C speed and keep its execution engines agreeing. Use before finishing ANY change to core/evaluator (bytecode.rs, nguvu/, aot.rs, native.rs, eval/ops.rs, eval/methods.rs, builtins/*), to parser lowering/desugaring (parse.rs), to the .asb format, or to examples/sudoku — and whenever adding an operator, builtin, method, opcode, Expr/Stmt variant or keyword. Covers the architecture invariants, the required tests, the Sudoku benchmark and its thresholds, and how to debug a regression.
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
| Typed register VM | `core/evaluator/src/bytecode.rs` | `.asb` bytecode artifacts without native code (`ASILI_AOT=0`, a platform without a backend, wasm) |
| Native code (`nguvu`) | `core/evaluator/src/nguvu/` (+ `native.rs` analysis, `aot.rs` hash/ABI) | `pata jenga` writes `kilele/<name>.nguvu`; `pata tenda`/`jenga --tenda`/runner map it if its hash, ABI, architecture and CPU features match |

No external tool is involved anywhere: not to build `pata`, not in `pata jenga`, not to run.

`nguvu` pipeline: `lower.rs` (bytecode → typed IR; unrolls small constant-bound loops) →
`opt.rs` (constant folding, `range.rs` interval analysis, if-conversion, bit-test/popcount
fusion, value reuse, constant hoisting, liveness DCE) → `regalloc.rs` + `schedule.rs` (shared
by targets) → `codegen.rs`/`x64.rs` (x86-64) or `codegen_a64.rs`/`a64.rs` (AArch64) →
`mem.rs` (executable mapping). A change to IR semantics or a new IR instruction needs both
code generators; run the differential tests on arm64 too (CI's `native-arm64` job, or locally:
`CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="qemu-aarch64-static -L /usr/aarch64-linux-gnu"
cargo test -p asili-evaluator --target aarch64-unknown-linux-gnu`).

## Invariants — never break these

1. **One implementation of every semantic rule.** Operators, casts, `?`/`jaribu`, truthiness,
   indexing, formatting, iteration and every method live in `eval/ops.rs` / `eval/methods.rs`;
   numeric opcode semantics live in `bytecode.rs::numeric_op` (used by both the interpreter loop
   and `exec_slow`). The VM and AOT call these; they never re-implement them. Generic-`Value`
   instructions in native code call back into `Vm::exec_slow` through the runtime ABI table.
   If you find yourself writing a second `match` over `BinaryOp` semantics, stop and call the
   shared function instead.
2. **One native backend.** `nguvu`, in-house, ahead of time. No external compiler/assembler/
   linker, no C transpiler, no JIT. (A Cranelift JIT and an LLVM IR → clang backend were both
   removed on purpose; see commit `d77a8c4` and the history of `aot.rs` if you need them.)
   Every IR transform must keep bit-identical results; an optimization that can't prove its
   precondition leaves the code alone.
3. **Native code is bit-identical to the interpreter.** A `Namba` register may be lowered to
   `i64` only when `native::analyze_numbers` proves it whole, never NaN, never `-0.0`, and within
   ±2^53. Nothing is speculated: no guards, no deoptimization, native code never resumes the
   interpreter part-way through a call. If a counter needs to be an integer for speed, make the
   analysis prove it (square-root narrowing, loop-accumulator caps are the existing tools).
   `sdiv`/`srem` only on proven integers. Bounds checks
   are dropped only for indices in `NumAnalysis::safe_index`. When in doubt, the analysis must
   return the conservative fact (TOP), never an optimistic one.
4. **Every opcode is described to the analysis.** A new `Opcode` needs entries in
   `native::num_reads`, `num_writes` and `list_writes`, and a `transfer` rule (or it falls to the
   conservative default — check that it does). Missing a write means the analysis believes a
   stale fact: silent miscompilation.
5. **Formats are versioned.** Changing `Opcode`'s serialized shape → bump `BYTECODE_VERSION`
   in `asb.rs`. Changing the runtime ABI struct, a native function signature, or anything the
   generated code assumes about the VM → bump `ABI_VERSION` in `aot.rs`. Changing what code
   `nguvu` generates → bump `IMAGE_VERSION` in `nguvu/mod.rs`. A stale image must be rejected,
   never loaded.
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
  native code; values *and error messages* must match. Add a snippet for every new operator, method,
  builtin or syntax form.
- `cargo test -p asili-evaluator --test native_tiers` — interpreter vs native code (through the
  on-disk image), bit-for-bit (NaN bit patterns excluded: Asili can't observe them).
  Add edge cases for anything numeric: `-0.0`, NaN, ±∞, ±2^53 and beyond (must stay floats),
  negative `%`/`//`, shifts outside `0..=63`, out-of-range indices.
- `cargo test -p asili-evaluator --test bytecode` — compiler/VM unit behaviour.
- A new `nguvu` transform needs a snippet that exercises it — check by breaking the transform
  on purpose and watching the test fail (a test that still passes covers nothing).

## The benchmark — run it, don't guess

```bash
examples/sudoku/bench/run.sh 7      # best-of-7 whole-process wall time
```

It builds release `pata` and the standalone runner, runs `pata jenga --namna release` on the
example (which fails if the program falls back to the tree-walker or native code can't be
built), and checks every implementation reports `Majaribio: 90665` before timing it.

Reference (2026-09, this container, standalone runner): gcc C 8.0 ms · clang C 6.4 ms · Rust
7.0 ms · **asili-nguvu 7.1 ms** · asili-vm 111 ms · Python 304 ms. Process start-up is ~3.3 ms
of every figure here; solve-only (run minus an empty run): nguvu ≈ 3.0 ms, clang -O2 C
≈ 3.1 ms, gcc -O2 C ≈ 4.7 ms.

**Thresholds** — treat any of these as a regression to fix before finishing:
- asili-nguvu slower than clang C on the solve, or more than ~1 ms slower than its previous
  figure;
- asili-vm above ~130 ms;
- any "wrong result" (attempt count ≠ 90,665) — that is a correctness bug, not noise.

Timings on shared machines jitter by a millisecond or two; rerun before concluding. Record the
new figures in `docs/design/performance.md` when they change meaningfully.

## Debugging a regression

1. `ASILI_AOT=0` vs default: if only native code slowed down, it's `nguvu` or the analysis; if
   both did, it's the bytecode compiler or VM. If native code is barely faster than the VM,
   check that `analyze_numbers` converged: a function it gives up on (100,000 steps) runs
   entirely on floats — `ASILI_NGUVU_IR` shows `FAdd`/`FloatToIntSat` where `Add` was expected.
2. Did the program still lower to bytecode? `compile_module_explained` reports the first
   construct that forced the AST fallback (a 30× slowdown looks exactly like this).
3. `ASILI_NGUVU_IR=<file>` dumps the optimized IR with register locations and loop depth;
   `ASILI_NGUVU_DUMP=<file>` writes the machine code (`objdump -D -b binary -mi386:x86-64`),
   its function offsets and load address (`.offsets`/`.base`, to line up with profiler
   addresses). Both need an in-memory compile: `ASILI_NGUVU=1` with the `.nguvu` moved away.
   Look for runtime `Call`s (`Exec` spills) in hot blocks, `FloatToInt`/`IntToFloat` pairs where
   a register should have stayed integer, and spilled (`mem`) registers at high depth.
4. Instruction counts beat wall time for small deltas:
   `valgrind --tool=callgrind target/release/pata-cli tenda examples/sudoku/kilele/sudoku.asb`
   (nguvu generated code ≈ 24M instructions; whole clang C program ≈ 27M). Add
   `--dump-instr=yes` and join addresses with the dump for per-instruction costs.
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
