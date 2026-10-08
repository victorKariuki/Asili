# Performance: from tree-walker to native code

How Asili executes programs fast, why it is built this way, what the audit that started this
work found, and where the remaining gaps are. Benchmark: the Arto Inkala "world's hardest"
Sudoku solved by the iterative MRV backtracking solver in `examples/sudoku` (90,665 candidate
attempts, 10,041 backtracks), with identical C, Rust and Python ports in
`examples/sudoku/bench/`.

## Results

One core, best of 21–61 runs (September 2026, this repository's cloud container). "Solve only"
is a program's process time minus an empty program started the same way (process start-up in
this container is ~3.3 ms for anything).

| Engine | Solve only | Whole process |
|---|---|---|
| **Asili native (`nguvu`)** | **≈ 2.6 ms** | **7.2 ms** |
| C, clang -O2 | ≈ 3.1 ms | 6.5 ms |
| C, gcc -O2 | ≈ 4.7 ms | 8.1 ms |
| Rust -O | — | 6.8 ms |
| Asili register VM (removed) | ~110 ms | 118 ms |
| Asili tree-walker (today's fallback, no native backend) | — | ≈ 945 ms |
| Python 3 | — | 302 ms |
| Asili LLVM AOT (retired) | 4.8 ms | 10 ms |
| Asili tree-walker (before) | 3.4 s | — |
| Asili stack VM (before) | 1.5 s | — |

The Asili solve is faster than clang-compiled C. Whole-process figures depend on how the
runner is built: the default dynamically linked release build starts ~0.9 ms slower than a C
program, but the shipping build (`cargo build --profile dist -p asili-runner --target
x86_64-unknown-linux-musl`: fat LTO, `panic = "abort"`, static non-PIE, no dynamic loader)
starts in ~0.7 ms — faster than an empty dynamically linked C program (1.2 ms) — and runs the
whole Sudoku in 6.0 ms against 6.5 ms for clang C, static or dynamic (`run.sh`, which builds it
that way when the musl target is installed). By callgrind instruction count the Asili run executes 24.0M
instructions in total (22.8M in native code and the runtime helpers it calls) against 27.0M
for the clang C program and 43.8M for gcc's. The Asili figures need no C compiler anywhere:
`pata jenga` writes the machine code itself.

Reproduce with `examples/sudoku/bench/run.sh` (standalone runner, both C compilers, 0.1 ms
resolution).

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

## Beyond Sudoku: second audit (September 2026)

Six small programs, each with a line-for-line C port (`gcc -O2`), whole-process best of 9 on the
`dist`/musl runner:

| Program | What it stresses | C | Before | Asili native | VM (removed) | Native / C |
|---|---|---|---|---|---|---|
| `fib(32)` | 7M scalar calls | 12 ms | 1706 ms | 26 ms | 306 ms | 2.2× |
| Mandelbrot 400×300 | float loops | 21 ms | 33 ms | 27 ms | 280 ms | 1.3× |
| Sieve to 5M | memory bandwidth | 18 ms | 102 ms | 20 ms | 187 ms | 1.1× |
| 20M-step modular loop | integer division | 84 ms | 124 ms | 69 ms | 644 ms | 0.8× |
| Bubble sort, 3,000 | branchy list code | 4 ms | 6 ms | 5 ms | 158 ms | 1.3× |
| 200k string builds | generic values | 8 ms | 237 ms | 17 ms | 16 ms | 2.0× |

("Before" is the first measurement of this audit; the other columns were re-measured together
on one machine after the later work below.)

Generic-value workloads (no C port; whole process, native / VM):

| Program | What it stresses | Before | Now |
|---|---|---|---|
| 100 `umbo` particles × 2,000 steps | struct values, field access | 4,945 ms (tree-walker) | 81 / 86 ms |
| 1,000-word `Orodha<Neno>` summed 5,000× | `kwa … katika`, `Neno` copies | 743 ms | 130 / 128 ms |
| `Kamusi` build and lookups | maps with computed keys | 577 ms | 67 / 69 ms |

What it found and what changed:

1. **Calls between `kazi` went through the interpreter** from native code (a full VM frame per
   call). Scalar functions now get a second, direct native entry (`CallDirect`), keeping the
   VM's depth limit and a stack-headroom check. On musl's main thread stacker sees only the
   committed ~128 KB of stack, which silently sent every direct call to the slow path until the
   VM kept the headroom free (on a fresh segment) before entering native code.
2. **Call sites stored every live value and argument** around every call; they now skip
   values whose stack slot is already current and rematerialize constants. Frames give slots
   only to values that can need one.
3. **`reuse_values` was block-local**, so a branch arm reloaded the elements its test had just
   compared. It now runs over extended basic blocks, forwards stored list elements, and looks
   through copies.
4. **Division by constants ≥ 2048 used `div`** (tens of cycles). `div_magic` covers every
   divisor for the ±2^53 integer range, on AArch64 too (`umulh`).
5. **Generic strings paid for copies**: concatenation copied both operands, `urefu` ran Unicode
   segmentation on ASCII, number formatting went through float printing, and the VM cloned
   method receivers (and had its own copy of `urefu`).
6. x86-64 memory operands always carried 32-bit displacements. Loop-head alignment was tried
   and rejected: it slowed the Sudoku run by 3 %.

Later rounds made `Neno`, `Orodha` and `umbo` values shared (`Rc`, copied only when written),
loaded typed `umbo` fields into numeric registers, gave `kwa … katika` a one-instruction item
read, and fixed the static runner's stack handling (the VM's `fib(32)` spent 5–12 s mapping
stack segments on musl's main thread). What is left: `fib` still passes arguments and results
through memory and keeps the VM's depth count on every call (an experiment passing them in
registers measured slower; dropping the depth traffic entirely would gain only ~7 %); every
generic-value instruction runs in the host (`host.rs`). See *Remaining gaps*.

## Techniques, and how Asili uses them

**Typed register bytecode.** (It was interpreted by a register VM until October 2026; now it is
only the native backend's input.) Stack VMs spend most of their time moving operands; register VMs (Lua 5,
Dalvik, wasm3) name operands in the instruction, roughly halving instruction count. Asili goes
further and types the register files: `nums` (`f64`, also holding `Ukweli` as 0/1), `lists`
(`NumList` for `Orodha<Namba>`: integer words while every element is an exact integer, `f64`
bits otherwise), `vals` (generic `Value`). Types come from annotations,
literals and signatures at compile time, so numeric instructions never inspect a tag.
Superinstructions fuse the hottest pairs: compare-and-branch (`JumpIfNot`), increment-and-loop
(`ForStep`), `b[i]?` as one bounds-checked load (`ListGet`). Constants live in preloaded
registers.

**One source of truth for semantics.** Every operation on a generic `Value` — operators, casts,
methods, `?`/`jaribu`, iteration, display — is a function in `eval/ops.rs` / `eval/methods.rs`
that both the tree-walker and native code's host (`host.rs`) call; `host.rs::numeric_op` is
the reference numeric semantics native code must match bit for bit. Differential tests (`tests/engines_agree.rs`) run each
snippet on every engine and require identical values and error text.

**Native code: one in-house backend, `nguvu`.** `pata jenga` compiles each bytecode function
to machine code itself — no C source, no LLVM, no external compiler, assembler or linker — and
writes it next to the `.asb` as `<name>.nguvu`, stamped with the bytecode hash, ABI and image
versions, architecture and the CPU features it relies on, so a stale or foreign image is never
mapped; when it is missing or stale the runner compiles in memory. Generic-value instructions
call back into the host's single-step function (`Host::exec_slow`) through a small runtime
table, spilling and reloading only the registers that instruction touches, so native code never
changes behaviour. There is no bytecode interpreter: without a backend for the platform (or
with `ASILI_AOT=0`) the tree-walker runs the syntax tree every bytecode artifact carries.

Pipeline (`core/evaluator/src/nguvu/`):
- `lower.rs`: bytecode → a typed IR of virtual registers (integer or float class, chosen by the
  range analysis below); small counted loops with constant bounds are fully unrolled so the
  counter is a constant in each copy.
- `opt.rs` + `range.rs`: constant folding (with block-local knowledge), an interval analysis
  over the IR (widening at loop heads, then narrowing) that deletes comparisons and overflow
  guards it proves, if-conversion of small branches to conditional moves, `select(c, v+1, v)` →
  `v + c`, bit-test fusion, summed single-bit tests → `popcnt`, division by a constant of a small
  proven range → multiply-and-shift (`p // 9` → `(p * 57) >> 9`, multiplier verified for every
  dividend), local value reuse, constant hoisting, liveness-based dead-code elimination.
- `regalloc.rs` + `schedule.rs` (shared by targets): priority allocation over precise live
  ranges with copy coalescing and callee-saved preference across calls; compares fused into
  branches and conditional selects.
- `codegen.rs`/`x64.rs` (x86-64, System V or Windows x64) and `codegen_a64.rs`/`a64.rs`
  (AArch64 AAPCS64):
  instruction selection and encoding; `mem.rs` maps the code executable (never writable and
  executable at once).

History: a Cranelift JIT prototype (~40 ms) and then an LLVM-IR → clang backend (4.8 ms solve,
clang required at build time) preceded `nguvu`; both were removed so there is exactly one
native backend to keep correct (recoverable from commit `d77a8c4` and the history of `aot.rs`).

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
`i64` — bit-identical to the `f64` semantics. A list register whose elements are all proven such
integers is kept in integer words and read with plain integer loads. `sakafu(a / b)` on non-negative integers becomes
`sdiv` (below 2^53 the rounded quotient never crosses the next integer), `%` on integers becomes
`srem`, and list accesses whose index is proven in range drop their bounds check.

**No speculation.** Native code never guesses: a register is `i64` only when the analysis
proves it, and a register it cannot bound stays `f64` (an exact float add, as fast as an integer
one for a counter). There are no guards, no deoptimization and no resuming the interpreter
part-way through a call — native code either finishes the call or stops with the call's error.
Two refinements keep the counters that used to need a guess provable:

- *square roots* — after `i * i <= n` (both edges), `i` is bounded by `±(⌊√n.hi⌋ + 1)`;
- *loop accumulators* — a register that only grows by non-negative increments inside a
  bounded `kwa` loop (and is reset before it) is capped at `reset + trips × max increment`,
  derived from the previous round's proven facts and re-analysed until the caps settle (at
  most three rounds). This bounds Sudoku's attempt counter and the sieve's `hesabu`.

A direct native-to-native call that is too deep, or would run out of native stack, goes through
the host's own call path (`RtFn::CallHost`), which grows the stack, reports the depth error, and
still runs the callee's native code. Loop headers are aligned to 32 bytes (one x86-64 fetch
window), so a loop's speed no longer depends on where unrelated code shifted it.

## Correctness guardrails

The full checklist — invariants, required tests, benchmark thresholds, how to debug a
regression — is the `performance-guardrails` skill (`.claude/skills/performance-guardrails/`).

- `tests/engines_agree.rs`: tree-walker vs native code, values and error messages
  (including list representations: `-0.0` in an integer list, huge integers, lists switching
  to floats, lists across calls).
- `tests/native_tiers.rs`: tree-walker vs native code (through the on-disk image) bit-for-bit
  on the numeric edge cases (`-0.0`, NaN, ±∞, 2^53, negative `%` and floor division, shifts
  outside `0..=63`, out-of-bounds reads and writes, recursion, labelled loops, callbacks,
  unrolled loops with `vunja`/`endelea`, popcount, small-range division, the full Sudoku).
  Every optimization has been checked by breaking it on purpose and watching a test fail.
- Both run on x86-64 and on AArch64 (CI's `native-arm64` job).
- NaN *bit patterns* are the one thing not compared: Asili cannot observe them.

## Plan and status

The plan this work followed, in order, and where each step stands:

1. Measure honestly — fix the fixture, add identical C/Rust/Python solvers and `run.sh`. Done.
2. Typed register VM with fused instructions and unboxed register files. Done (1.5 s → 0.11 s).
3. One implementation of the language's semantics shared by every engine, with differential
   tests (`engines_agree.rs`, `native_tiers.rs`). Done.
4. Native code without a C step: LLVM IR text → clang, AOT at `pata jenga`, hash-checked at load.
   Done, then superseded by step 9; the Cranelift JIT prototype was removed earlier.
5. Integer range analysis, speculation with deoptimization, bounds-check elimination. Done
   (4.8 ms solve vs 4.4 ms for gcc C with the LLVM backend). Speculation was later removed
   (step 11).
6. Terse syntax that lowers to the fast forms: `//`, `%= &= |= ^= //=`, compound assignment on
   list elements, bitwise-before-comparison precedence, `a[i]` returning the element. Done.
7. Codebase-wide deduplication so nothing is implemented twice (compile front end, keyword list,
   expression walk, builtin helpers, `pata.toml` discovery, artifact running, …). Done; the
   single sources are listed in `CLAUDE.md` ("Write once, reuse").
8. Guardrails: the `performance-guardrails` skill and `CLAUDE.md` rules make the invariants,
   tests and benchmark thresholds part of finishing any engine change. Done.

9. An in-house backend, `nguvu` (x86-64), replacing clang: register allocation, then loop
   unrolling, interval analysis, if-conversion and popcount, integer lists and range-aware
   division. Done — solve ≈ 2.6 ms vs clang C ≈ 3.1 ms; `pata jenga --namna release` needs no
   external tool.
10. The LLVM/clang backend removed; `nguvu` for AArch64. Done.
11. Stop guessing: speculation and deoptimization removed from `nguvu`, replaced by stronger
    proofs (square-root narrowing, loop-accumulator caps) and a host call path for deep direct
    calls. Done — every benchmark at parity or faster (sieve 20.7 → 18.7 ms, Sudoku whole
    process 2.3 ms, solve below clang `-O2` C).
12. Remove the VM: bytecode only runs as native code (compiled in memory when no image was
    built); the tree-walker is the fallback where there is no backend, with the same
    10,000-call depth limit as native code and no nesting limit. Done — native speed
    unchanged; the fallback is ~10× slower than the VM was (Sudoku ≈ 0.95 s).

Next steps are the "Remaining gaps" below.

## Knobs

- `ASILI_AOT=0` — don't build (at `pata jenga`) or load or compile (at run time) native code:
  the tree-walker runs the program.
- `ASILI_BYTECODE_DUMP=1` (at `pata jenga`) — print every compiled `kazi`'s instructions.
- `ASILI_NGUVU_IR=<file>` / `ASILI_NGUVU_DUMP=<file>` — dump the optimized IR with register
  locations / the machine code, function offsets and load address (in-memory compiles).

## Remaining gaps

- There is no mixed mode: a program lowers to bytecode as a whole or not at all
  (`compile_module_explained` names the first `kazi` and line that blocked it). Pattern `weka`,
  computed module constants (an init function the host runs once), maps with computed keys and
  method calls on a receiver whose type is only known at run time all lower. The one construct
  that does not is `tupa` of a binding declared outside an enclosing loop (the tree-walker fails
  on the loop's second pass, which a static drop cannot reproduce). A program that does not lower
  still builds a syntax-tree artifact under `--namna dev`; `--namna release` refuses it.
- Threads (`tenda`) and server workers (`mkondo_tumikia`, `mkondo_tumikia_http`) share the
  bytecode program and its native code (`spawn::Shared`) and build one host per thread; only a
  program built as a syntax-tree artifact still runs them on the tree-walker.
- Native-to-native calls are direct only for scalar (`Namba`/`Buliani`) functions without
  lists or generic values; others go through the host's call path. Direct calls pass
  arguments as `f64` through a memory buffer, so `fib(32)` is ~40 ms against C's ~11 ms;
  arguments and results in registers, and inlining small helpers (`sanduku_la(r, c)`), would
  close most of that.
- Lists store integers in 1, 2, 4 or 8 bytes (`numlist.rs`), and native code uses 1-, 4- or
  8-byte elements chosen from the range analysis, so the sieve now moves bytes like C (23 ms
  against 17 ms). The rest of that gap is the host call that builds the list
  (`orodha_rudia`) and the bounds checks the analysis cannot drop.
- `Neno`, `Orodha` and `umbo` values are shared (`Rc`) and copied only when written, and the
  static runner has its own allocator and memory primitives (musl's locked on every allocation
  and copied bytewise): string building went from 99 to 17 ms (C: 10 ms). Every generic value
  operation still runs in the host, one `exec_slow` dispatch per instruction.
- The native image is mapped at start-up by the runner, from a file beside the artifact or from
  a standalone executable (`pata jenga --namna release` appends both to the runner). `pata tenda` itself (the full toolchain binary) starts ~3 ms slower than the
  runner; ship programs with the `dist`-profile static runner.
- `list_push`/`list_remove` are runtime calls (~1M instructions on the benchmark); inlining the
  common case needs a list layout native code may write directly.
- Platforms: x86-64 (System V and Windows x64) and AArch64 (Linux, macOS) are supported;
  Windows on ARM64, 32-bit targets and wasm run on the tree-walker (≈ 10× slower than the
  removed VM was; a backend or a faster fallback would close it). A hardened-runtime macOS app
  needs the `com.apple.security.cs.allow-jit` entitlement for native code (without it, the
  tree-walker).
