# `pata jenga`

Purpose: build project sources into bytecode or target binary. Optionally display workspace information.

Flags:
- `--workspace-info`: Display workspace members from `pata.toml`'s `[eneo-kazi]` table and exit
- `--tenda`: After building, execute `kuu` with any trailing arguments
- `--pato <path>`: Output directory (default `kilele/`)
- `--lengo <lengo>`: Build target (default `native`); overrides `pata.toml`'s `[jenga] lengo`
- `--namna <dev|release>`: Build profile. `dev` (default) is best effort: bytecode when the
  program benefits, native code where the platform has a native backend, otherwise the VM (or
  tree-walker) with a note. `release` requires both: every program is compiled to bytecode (a
  construct the VM can't lower yet fails the build, naming the `kazi` and line) and its native
  code must be built (a platform without a backend fails the build); release never uses the
  build cache. `embedded` is
  rejected as not implemented yet, and unknown names are rejected
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
- for a bytecode artifact, compiles it ahead of time to native machine code with Asili's own
  backend (`nguvu`; no external compiler, assembler or linker) into `<name>.nguvu` beside the
  `.asb`, printing `msimbo asilia: <path>`. On a platform without a backend it prints
  `msimbo asilia haukujengwa (...); kilele kitaendeshwa na VM` and the artifact runs on the
  register VM — the build does not fail. `ASILI_AOT=0` skips this step. Leftover `<name>.so`/
  `<name>.ll` files from the retired LLVM backend are removed.
- `--tenda` runs the artifact just built exactly as `pata tenda` would (native code when present)

Failures:
- syntax/type/ownership diagnostics
- dependency resolution failures
- target backend not available
