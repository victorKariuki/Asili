# Tool qualification evidence

What a manufacturer needs to assess the Asili toolchain as a development tool (ISO 26262-8
§11, DO-330 in spirit) and, for its runtime and device runtime, as software of unknown
provenance (SOUP, IEC 62304 §8): what the tool does, how errors in it could reach a product,
the evidence that it works, and its known limits. It supports — it does not replace — the
manufacturer's qualification: the product's intended use, its classification and the final
judgement are the manufacturer's.

## The tool

| Item | Role in a product | Kind |
|---|---|---|
| `pata` (CLI), `nguvu` (code generator) | Turns Asili source into the code the product runs: bytecode (`.asb`), native images (`.nguvu`), standalone executables, Cortex-M objects (`-cortex-m.o`). | Development tool whose output is in the product |
| Runtime (`asili-evaluator`: host, builtins, allocator, safe state) | Linked into standalone executables and the runner; runs non-native operations and failure handling. | SOUP in the product |
| Device runtime (`asili-kifaa`) | Linked into firmware with Cortex-M objects: list access, `fmod`/`pow`, conversions. | SOUP in the product |

## How an error in the tool could reach a product

The tool's output is executable code, so an error in it can introduce a defect directly (tool
impact TI2 in ISO 26262 terms). The hazards and their controls are in
[risk-register.md](risk-register.md) (H-1, H-5, H-6, H-7 above all).

## Evidence

Each claim is a requirement in [requirements.md](requirements.md); which tests verify it is in
[traceability.md](traceability.md), kept current by a test.

| Measure | What it shows | Where |
|---|---|---|
| Reference results | Every operator, conversion and method on edge values (−0, NaN, ±∞, beyond 2^53, negative remainders, shifts out of range) gives hand-checked results bit for bit. | `tests/golden/`, `native_tiers.rs`, `engines_agree.rs` (REQ-NUM-1, -2) |
| IR verification | Every function's intermediate code is well formed after lowering and after optimization; each optimizer pass is checked in debug builds. | `nguvu/verify.rs` (REQ-COMP-1) |
| Differential fuzzing | Random programs give identical results with and without the optimizer (300 per test run, 2,000 per CI run; 20,000 run during development without a difference). | `tests/fuzz.rs` (REQ-COMP-2) |
| Target agreement | The browser build and the Cortex-M build compute what native code on the build machine computes, bit for bit (examples, hand-written cases, random strict programs). | `agree.sh`, `tests/kifaa.rs` (REQ-TGT-1, -2) |
| Encoding checks | Every Cortex-M instruction encoding equals the LLVM assembler's. | `nguvu/t32.rs` (REQ-TGT-4) |
| Reproducibility | The same source builds byte-identical artifacts. | `builds_are_reproducible` (REQ-COMP-3) |
| Load-time verification | Damaged bytecode and mismatched native images are rejected before they run. | `bytecode_verify.rs`, `native_tiers.rs` (REQ-COMP-4, -5) |
| Strict-code checks | The bounded subset is enforced; step and memory bounds are reported; strict code does not allocate. | `salama.rs`, `salama_memory.rs` (REQ-STRICT-1..3) |
| Failure handling | Errors, an unfed watchdog and the memory limit enter the safe state exactly once; the runtime has no panic paths of its own. | `pata/runner/tests/hali_salama.rs`, CI clippy step (REQ-FAIL-1..6) |
| Continuous integration | All of the above on every change, on x86-64 Linux, AArch64 Linux, Apple silicon macOS and x86-64 Windows, plus the wasm and Cortex-M jobs. | `.github/workflows/ci.yml` |

## Limits

- **No formal proof.** Correctness rests on tests, fuzzing and verification of the intermediate
  code, not on a proved compiler. The fuzzer covers the numeric subset (arithmetic, bitwise
  operators, comparisons, branches, counted and `wakati` loops, lists, calls); text, maps and
  user types are covered by the reference tests only.
- **The language specification is prose** (`docs/spec/`); the reference results are hand-checked
  outputs, not derived from a formal semantics.
- **Shared parts.** Optimized and unoptimized code share lowering and code generation, so the
  differential fuzzer cannot find a bug in those shared parts that gives the same wrong answer
  both ways; the reference results and target agreement are the checks there.
- **Bounds are computed, not measured.** The step bound counts bytecode steps; the time a step
  takes on a given processor is the product's to measure.
- **Device scope.** The Cortex-M target compiles strict code only, as a library; start-up code,
  interrupt handling and RTOS integration are the firmware's. `**` uses `libm` on the device.

## Using the tool in a product

1. Fix the toolchain version (a release commit on `develop` or `main`) and record it with the
   product's configuration.
2. Write every safety function as strict code (`#[salama]`) and declare a safe state
   (`#[hali_salama]`); keep the bounds `pata jenga` prints in the design records.
3. Build with `--namna release`; record the hashes of the `.asb`, `.nguvu`, executable or
   `-cortex-m.o` artifacts, and check that a rebuild from the same source gives the same bytes.
4. Verify the product's functions on the target hardware with the product's own tests, and
   measure worst-case execution time there.
5. Before each release, review the toolchain's open anomalies (below) against the product.

## Known anomalies

Open defects are tracked on the project's GitHub board "Asili bug tracker"
(`github.com/users/victorKariuki/projects/16`) and in this repository's issues. Fixed defects are
listed under "Fixed" in `CHANGELOG.md` for each release.
