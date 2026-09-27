# `pata jenga`

Purpose: build project sources into bytecode or target binary. Optionally display workspace information.

Flags:
- `--workspace-info`: Display workspace members from `pata.toml`'s `[eneo-kazi]` table and exit
- `--tenda`: After building, execute `kuu` with any trailing arguments
- `--pato <path>`: Output directory (default `kilele/`)
- `--lengo <lengo>`: Build target (default `native`); overrides `pata.toml`'s `[jenga] lengo`
- `--namna <profile>`: Build profile (dev/release/embedded) — not yet functional
- `--muda`: Print phase-latency timings (`kuchanganua` = compile, `kutoa` = emit) after a
  successful build, via `pata_cli::pipeline::performance::PerformanceMetrics`/`ScopedTimer`.
  Works on both the normal build path and the single-file cache-hit early-return path.

Success:
- parses `pata.toml` (or builds single file without manifest)
- compiles from `[chanzo].kuingia`
- emits a register-bytecode `.asb` artifact (everything except `linganisha`, `tupa`, pattern
  `weka`, map/struct literals, enum construction and field access, for which the
  serialized-AST artifact is emitted instead), plus a `.build.manifest` sidecar under the output
  directory
- for a bytecode artifact, compiles it ahead of time to native code: LLVM IR (`<name>.ll`) built
  by `clang -O2` into `<name>.so`/`.dylib`/`.dll` beside the `.asb`, printing
  `msimbo asilia: <path>`. Without `clang` it prints `msimbo asilia haukujengwa (...); kilele
  kitaendeshwa na VM` and the artifact runs on the register VM — the build does not fail.
  `ASILI_AOT=0` skips this step; `ASILI_CLANG=<path>` picks the compiler. `clang` is needed only
  where `pata jenga` runs.
- `--tenda` runs the artifact just built exactly as `pata tenda` would (native code when present)

Failures:
- syntax/type/ownership diagnostics
- dependency resolution failures
- target backend not available
