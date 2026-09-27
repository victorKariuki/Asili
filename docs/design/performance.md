# Performance: from tree-walker to native code

How Asili executes programs fast, why it is built this way, what the audit that started this
work found, and where the remaining gaps are. Benchmark: the Arto Inkala "world's hardest"
Sudoku solved by the iterative MRV backtracking solver in `examples/sudoku` (90,665 candidate
attempts, 10,041 backtracks), with identical C, Rust and Python ports in
`examples/sudoku/bench/`.

## Results

Solve time on one core, best of several runs:

| Engine | Solve only | Whole process |
|---|---|---|
| C, clang -O2 | 3.2 ms | — |
| C, gcc -O2 | 4.4 ms | 7 ms |
| Rust -O | — | 7 ms |
| **Asili AOT (LLVM)** | **4.8 ms** | **10 ms** |
| Asili register VM | ~110 ms | 115 ms |
| Python 3.11 | — | 306 ms |
| Asili tree-walker (before) | 3.4 s | — |
| Asili stack VM (before) | 1.5 s | — |

By callgrind instruction count, the AOT solve executes ~30M instructions against ~27M for
clang-compiled C. The remaining whole-process gap is mostly `pata`'s own startup (~3 ms).

Reproduce with `examples/sudoku/bench/run.sh` (whole-process figures above: best of 7,
re-measured after the codebase-wide deduplication pass, September 2026).

## Audit findings (September 2026)

1. **The benchmark measured the wrong puzzle.** The example's board literal had 82 cells (an
   extra `0` in the last row), so it solved an easier puzzle (7,145 attempts). That is where the
   0.6.0 "0.05 s bytecode" figure came from; on the real puzzle the old stack VM took 1.5 s.
2. **The old VM could not finish the example**: it crashed on `vipande`/`ramani`/`jiunge`
   (`bytecode method haijaungwa mkono`) when printing the board, because the compiler emitted
   generic method calls the VM did not implement instead of falling back.
3. **Semantics had drifted between the two engines**: non-short-circuit `na`/`au` in the VM,
   different shift and cast rules, string comparisons numeric-only, `kila_na_fahirisi` arguments
   swapped, loop labels ignored, one slot per variable name across block scopes, user
   functions winning over same-named builtins.
4. **Boxing everywhere**: every VM operand was an enum carrying a full `Value`, so `a + b` meant
   two pops, two matches, two `Value` constructions and a push.
5. **`pata jenga --tenda` never ran the bytecode** — it re-interpreted the in-memory AST with the
   tree-walker.
6. Language: bitwise operators bound looser than `==` (the C wart), forcing parentheses in every
   bit test; no `|=`/`&=`/`^=`; no compound assignment on list elements.

## Techniques, and how Asili uses them

**Typed register VM.** Stack VMs spend most of their time moving operands; register VMs (Lua 5,
Dalvik, wasm3) name operands in the instruction, roughly halving instruction count. Asili goes
further and types the register files: `nums` (`f64`, also holding `Ukweli` as 0/1), `lists`
(`Vec<f64>` for `Orodha<Namba>`), `vals` (generic `Value`). Types come from annotations,
literals and signatures at compile time, so numeric instructions never inspect a tag.
Superinstructions fuse the hottest pairs: compare-and-branch (`JumpIfNot`), increment-and-loop
(`ForStep`), `b[i]?` as one bounds-checked load (`ListGet`). Constants live in preloaded
registers.

**One source of truth for semantics.** Every operation on a generic `Value` — operators, casts,
methods, `?`/`jaribu`, iteration, display — is a function in `eval/ops.rs` / `eval/methods.rs`
that both the tree-walker and the VM call; unboxed numeric opcodes share one
`bytecode.rs::numeric_op` between the interpreter loop and `exec_slow` (the path native code
calls back into). Differential tests (`tests/engines_agree.rs`) run each
snippet on every engine and require identical values and error text.

**Native code, two tiers.**
- *AOT through LLVM* (the tier that reaches C speed): `pata jenga` translates each bytecode
  function to LLVM IR text — no C source involved — and `clang -O2` optimizes and links it into
  a shared library next to the `.asb`. It embeds a hash of the bytecode, so a stale library is
  never loaded. Generic-value instructions call back into the interpreter's single-step function
  (`exec_slow`) through a small runtime table, spilling and reloading only the registers that
  instruction touches, so native code never changes behaviour.
- Without `clang` (it is needed only where `pata jenga` runs, not to build `pata` or to run a
  built program), the bytecode runs on the register VM. A Cranelift JIT prototype (~40 ms here)
  was removed so there is exactly one native backend to keep correct (recoverable from commit
  `d77a8c4`).

**Integer range analysis.** `Namba` is an `f64`; C and Rust use integers. Converting every bit
mask float → int → float, and serializing `n += 1` through 4-cycle `addsd` chains, cost 5× on
its own. `native.rs` runs a flow-sensitive interval analysis over the register bytecode:
- facts per register: range, whole-number, may-be-NaN, may-be-`-0.0`;
- branch refinement (inside `wakati v <= 9`, `v <= 9`; after `ikiwa bora == -1 { vunja }`,
  `bora != -1`);
- widening with thresholds (the program's constants and powers of two), so counters bounded by
  `81` and bit masks growing 1, 3, 7, … converge instead of jumping to infinity;
- element facts and lengths for `Orodha<Namba>` registers (`orodha_rudia(0, 9)` has length 9 and
  holds masks ≤ 511).

A register whose values are provably whole, never NaN/`-0.0` and within ±2^53 is stored as
`i64` — bit-identical to the `f64` semantics. `sakafu(a / b)` on non-negative integers becomes
`sdiv` (below 2^53 the rounded quotient never crosses the next integer), `%` on integers becomes
`srem`, and list accesses whose index is proven in range drop their bounds check.

**Speculation with deoptimization.** Some whole-number registers have no provable bound (the
candidate counter `n`, `majaribio`). They are still kept as `i64`, but every write checks
`|v| <= 2^53`; if a check ever fails, the native function writes every register back to the
frame and returns `STATUS_DEOPT` with the failing instruction, and the interpreter resumes the
same call there with exact `f64` semantics. This is the technique production JITs (V8, LuaJIT,
PyPy) use; `tests/native_tiers.rs` includes values crossing 2^53 to exercise it.

## Correctness guardrails

The full checklist — invariants, required tests, benchmark thresholds, how to debug a
regression — is the `performance-guardrails` skill (`.claude/skills/performance-guardrails/`).

- `tests/engines_agree.rs`: tree-walker vs VM vs AOT, values and error messages.
- `tests/native_tiers.rs`: interpreter vs AOT bit-for-bit on the numeric edge cases
  (`-0.0`, NaN, ±∞, 2^53, negative `%` and floor division, shifts outside `0..=63`,
  out-of-bounds reads and writes, recursion, labelled loops, callbacks, the full Sudoku).
- NaN *bit patterns* are the one thing not compared: Asili cannot observe them, and Rust and
  LLVM do not specify them (LLVM constant-folds `∞ − ∞` to a positive NaN, x86 produces a
  negative one).

## Plan and status

The plan this work followed, in order, and where each step stands:

1. Measure honestly — fix the fixture, add identical C/Rust/Python solvers and `run.sh`. Done.
2. Typed register VM with fused instructions and unboxed register files. Done (1.5 s → 0.11 s).
3. One implementation of the language's semantics shared by every engine, with differential
   tests (`engines_agree.rs`, `native_tiers.rs`). Done.
4. Native code without a C step: LLVM IR text → clang, AOT at `pata jenga`, hash-checked at load.
   Done; the Cranelift JIT prototype was removed to keep one native backend.
5. Integer range analysis, speculation with deoptimization, bounds-check elimination. Done
   (4.8 ms solve vs 4.4 ms for gcc C).
6. Terse syntax that lowers to the fast forms: `//`, `%= &= |= ^= //=`, compound assignment on
   list elements, bitwise-before-comparison precedence, `a[i]` returning the element. Done.
7. Codebase-wide deduplication so nothing is implemented twice (compile front end, keyword list,
   expression walk, builtin helpers, `pata.toml` discovery, artifact running, …). Done; the
   single sources are listed in `CLAUDE.md` ("Write once, reuse").
8. Guardrails: the `performance-guardrails` skill and `CLAUDE.md` rules make the invariants,
   tests and benchmark thresholds part of finishing any engine change. Done.

Next steps are the "Remaining gaps" below.

## Knobs

- `ASILI_AOT=0` — don't build (at `pata jenga`) or load (at run time) the native library.
- `ASILI_CLANG=/path/to/clang` — compiler used for AOT.

## Remaining gaps

- Programs using a construct the bytecode compiler does not lower (`linganisha`, `tupa`, pattern
  `weka`, map/struct literals, enum construction, field access) run entirely on the tree-walker.
  Per-function fallback, then lowering those constructs, would extend the fast path.
  `compile_module_explained` reports the blocking `kazi` and line.
- Calls between `kazi` from native code go through the interpreter's call path; inlining small
  numeric functions would let helpers like `sanduku_la(r, c)` cost nothing.
- The AOT artifact is a library loaded by `pata tenda`, not a standalone executable; that would
  need the runtime shipped as a static library.
- Values in `Orodha<Namba>` stay `f64` in memory (shared with the interpreter), so native code
  converts on load/store even when elements are proven integers.
