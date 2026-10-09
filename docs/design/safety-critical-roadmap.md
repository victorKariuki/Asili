# Safety-critical roadmap (later work)

Asili now runs every program as native code from its own backend (`nguvu`), at C speed on the
Sudoku benchmark, with no interpreter and no fallback. That is fast and deterministic in
*results*; it is not yet what life-support or other safety-critical software needs. This page
records what was missing, in the order it should be done, and where each item stands.

## 1. Guaranteed worst-case time per step

**Gap.** The runtime allocates while running (`Value` text, lists, maps, frames), keeps reference
counts (`Rc` drops can cascade), and grows its stack on demand (`stacker`). None has a time
bound, and timings so far are best-of-N, never worst case.

**Plan.**
- A *strict* build mode (`pata jenga --namna salama`, name to be decided) that rejects, at build
  time, in functions marked for it: allocation (generic `Value`s other than numbers and fixed
  numeric lists), recursion, loops without a static bound, and calls into the host that can
  allocate.
- Worst-case execution-time measurement for that subset: thousands of runs with tail
  percentiles (p99.9, max) under load, plus a static bound from instruction counts on the
  longest path of the lowered IR.
- The benchmark harness reports worst case beside best case.

**Status: done** (issue #77). `#[salama]` marks strict functions; `salama.rs` rejects
everything outside the subset at build time and `pata jenga` prints each one's bound on steps
per call (`hatua`) and frame memory (`kumbukumbu`). The benchmark reports worst beside best.

## 2. Fixed memory

**Gap.** There is no mode that allocates nothing after start-up.

**Plan.** In strict mode, every list has a declared capacity (`[0; 81]` already gives one),
frames are preallocated from the call graph (no recursion), and the host's pools are sized at
start-up; a run that would allocate fails the build, not the device. A memory-limit model (the
total a program may ever use) is computed and printed by the build.

**Status: done** (issue #78). Strict code allocates nothing while it runs: list copies reuse
storage and pooled frames keep their lists (`tests/salama_memory.rs` counts allocations).

## 3. Failure discipline

**Gap.** Nobody has audited the runtime for crash points (`panic!`, `unwrap`, `expect`, index
out of bounds in Rust), and there is no defined safe state, watchdog or memory-limit behaviour.

**Plan.**
- Audit `host.rs`, `native.rs`, `eval/`, `builtins/` for every panic path; turn each into an
  `EvalError` or prove it unreachable (`#![deny(clippy::unwrap_used, clippy::expect_used,
  clippy::panic)]` on the runtime crates, with documented exceptions).
- A defined safe state: a program declares a `#[salama]` handler the runtime calls on any
  unrecoverable error, deadline miss or memory-limit hit, before stopping.
- A watchdog hook: the host checks a deadline at loop back-edges in strict code.

**Status: done** (issue #79). `#[hali_salama]` is the safe state (run once on an error leaving
`kuu`, a runtime panic, the watchdog or the memory limit); `mlinzi_anza`/`mlinzi_lisha` are the
watchdog, `kikomo_kumbukumbu` the memory limit; the runtime modules deny `unwrap`, `expect`,
`panic!` and `unreachable!` (a CI clippy step).

## 4. Compiler assurance

**Gap.** Correctness rests on tests and comparison against recorded reference results
(`tests/golden/`), not on fuzzing or proof, and recent work still found real bugs (non-converging
range analysis, non-reproducible builds, resources released late).

**Plan.**
- Differential fuzzing: generate random well-typed programs (grammar-based), run them through
  native code on x86-64, AArch64 and wasm, and compare; shrink failures into `engines_agree`
  snippets.
- A reference interpreter kept *only as a test oracle* in the test tree (never shipped, never a
  fallback), written for clarity over speed, so fuzzing has an independent semantics.
- Translation validation for the riskiest passes (range analysis, if-conversion, unrolling):
  check each optimized function against its unoptimized IR on random inputs during tests.
- Reproducible builds checked in CI (same source → byte-identical `.asb` and `.nguvu`).

**Status: done** (issue #80), with one change of plan:
- *IR verifier* (`nguvu/verify.rs`): every function is checked after lowering and after the
  optimizer — registers and blocks exist, operand classes match, nothing is read before it is
  written on any path — and in debug builds after every single pass, naming the pass. It found
  if-conversion committing arm temporaries with a select that read an undefined register.
- *Differential fuzzing* (`tests/fuzz.rs`): random programs over the numeric subset (arithmetic,
  bitwise operators, comparisons, `ikiwa`, counted and `wakati` loops, list reads and writes,
  calls) run with and without the optimizer and must agree bit for bit, or fail with the same
  error. 300 programs in `cargo test` on every platform CI covers (x86-64, AArch64), 2,000 in a
  release CI step, any number with `ASILI_FUZZ_PROGRAMS`/`ASILI_FUZZ_SEED`; `agree.sh` also runs
  100 of them as wasm against native code. It found nested constant loops unrolling to hundreds
  of copies (3 s to compile a 40-line function; the unroll budget now counts nested loops at
  their unrolled size) and two quadratic compile-time passes (register allocation re-sorted
  every interval list per assignment; range analysis carried dead registers in every block
  state) — a 4,000-statement function went from 12 s to 0.8 s.
- *Translation validation* is the verifier plus optimized-against-unoptimized comparison on
  random programs, rather than evaluating IR on random inputs: an IR evaluator would be a second
  implementation of every instruction's semantics, which this repository avoids.
- *No reference interpreter as oracle*, for the same reason: the unoptimized pipeline and the
  hand-checked `tests/golden/` results are the references.
- *Reproducible builds*: `builds_are_reproducible` builds Sudoku and 20 fuzz programs four times
  each and requires byte-identical `.asb` and native images.
- *Load-time bytecode verifier* (`bytecode_verify.rs`, added): a damaged or edited `.asb` whose
  instructions name a register, jump target, function or constant that does not exist is
  rejected before native code runs it (native code reaches registers by address). The compiler's
  own output passes the same check.

## 5. Device targets

**Gap.** Native code runs on x86-64 and AArch64 desktops and servers, and as wasm in the
browser. There is no microcontroller or real-time OS backend.

**Plan.** A Cortex-M (Thumb-2) code generator beside `codegen_a64.rs`, a `no_std` host for the
strict subset (no allocator, no threads, no files), and a bare-metal runtime image; then an RTOS
port (Zephyr or FreeRTOS) for tasks and timing.

## 6. Regulatory process

**Gap.** Standards such as IEC 62304 (medical device software) need documented requirements,
traceability from requirements to tests, risk analysis (ISO 14971) and a qualified toolchain.
None of that exists.

**Plan.** A requirements document for the language and runtime (every rule in `docs/spec/` given
an ID), a traceability matrix from IDs to tests (generated from test annotations), a risk
register, and tool-qualification evidence for `pata` and `nguvu` (the fuzzing and validation in
§4 is the core of it).

## Order

Strict mode (§1–2) first: it turns "usually fast" into "fast or it won't build", and the failure
discipline (§3) and assurance work (§4) then have a small, well-defined subset to cover. Device
targets (§5) and the process work (§6) are separate projects that build on those.
