# Safety-critical roadmap (later work)

Asili now runs every program as native code from its own backend (`nguvu`), at C speed on the
Sudoku benchmark, with no interpreter and no fallback. That is fast and deterministic in
*results*; it is not yet what life-support or other safety-critical software needs. This page
records what is missing, in the order it should be done. None of it is started.

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

## 2. Fixed memory

**Gap.** There is no mode that allocates nothing after start-up.

**Plan.** In strict mode, every list has a declared capacity (`[0; 81]` already gives one),
frames are preallocated from the call graph (no recursion), and the host's pools are sized at
start-up; a run that would allocate fails the build, not the device. A memory-limit model (the
total a program may ever use) is computed and printed by the build.

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
