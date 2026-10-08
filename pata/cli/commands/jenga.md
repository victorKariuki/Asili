# `pata jenga`

Purpose: build project sources into bytecode or target binary. Optionally display workspace information.

Flags:
- `--workspace-info`: Display workspace members from `pata.toml`'s `[eneo-kazi]` table and exit
- `--tenda`: After building, execute `kuu` with any trailing arguments
- `--pato <path>`: Output directory (default `kilele/`)
- `--lengo <lengo>`: Build target (default `native`); overrides `pata.toml`'s `[jenga] lengo`
- `--namna <dev|release>`: Build profile. `dev` (default) is best effort: bytecode whenever
  the whole program lowers to it (whether or not it has loops), native code where the platform has a native backend, otherwise the tree-walker with a note; a `kazi`
  using a construct the bytecode compiler can't lower yet runs on the tree-walker while the rest of the
  program stays bytecode. `release` requires both: every program is compiled to bytecode (a
  construct the bytecode compiler can't lower yet fails the build, naming the `kazi` and line) and its native
  code must be built (a platform without a backend fails the build); release never uses the
  build cache. `embedded` is
  rejected as not implemented yet, and unknown names are rejected
- `--muda`: Print phase-latency timings (`kuchanganua` = compile, `kutoa` = emit) after a
  successful build, via `pata_cli::pipeline::performance::PerformanceMetrics`/`ScopedTimer`.
  Works on both the normal build path and the single-file cache-hit early-return path.

Success:
- parses `pata.toml` (or builds single file without manifest)
- compiles from `[chanzo].kuingia`
- emits a bytecode `.asb` artifact (everything except `linganisha`, `tupa`, pattern
  `weka`, map/struct literals, enum construction and field access, for which the
  serialized-AST artifact is emitted instead), plus a `.build.manifest` sidecar under the output
  directory
- for a bytecode artifact, compiles it ahead of time to native machine code with Asili's own
  backend (`nguvu`; no external compiler, assembler or linker) into `<name>.nguvu` beside the
  `.asb`, printing `msimbo asilia: <path>`. On a platform without a backend it prints
  `msimbo asilia haukujengwa (...); kilele kitaendeshwa bila msimbo asilia` and the artifact
  runs on the tree-walking evaluator (a bytecode artifact always carries the program's syntax
  tree for this) — the build does not fail. `ASILI_AOT=0` skips this step. When the image is
  missing or stale at run time on a supported platform, `tenda` compiles native code in memory
  instead. Leftover `<name>.so`/
  `<name>.ll` files from the retired LLVM backend are removed.
- `--tenda` runs the artifact just built exactly as `pata tenda` would (native code when present)
- `--namna release` also writes a standalone executable `<pato>/<name>` (`<name>.exe` on
  Windows), printing `programu huru: <path>`: the static runner `tenda` with the artifact and its
  native image appended, so `./kilele/<name> [hoja...]` runs the program directly with no other
  files and no `pata`. The runner is the `tenda` beside `pata` (`make install` puts it there) or
  the one `ASILI_TENDA` names; without one the build prints `programu huru haikujengwa: ...` and
  still succeeds

Failures:
- syntax/type/ownership diagnostics
- dependency resolution failures
- target backend not available
