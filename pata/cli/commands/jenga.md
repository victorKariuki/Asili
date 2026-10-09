# `pata jenga`

Purpose: build project sources into bytecode or target binary. Optionally display workspace information.

Flags:
- `--workspace-info`: Display workspace members from `pata.toml`'s `[eneo-kazi]` table and exit
- `--tenda`: After building, execute `kuu` with any trailing arguments
- `--fuatilia[=<namna>]`: Trace the build (phases `uchanganuzi`, `utatuzi`, `semantiki`,
  `bytecode`, `msimbo asilia`, and the parser's blocks) and, with `--tenda`, the run. `<namna>`
  is `mti` (default), `json`, `json:<faili>`, `mti:<faili>` or `binari:<faili>`; same as
  `ASILI_FUATILIA`. See `docs/design/tracing.md`.
- `--pato <path>`: Output directory (default `kilele/`)
- `--lengo <lengo>`: Build target (default `native`); overrides `pata.toml`'s `[jenga] lengo`
- `--namna <dev|release>`: Build profile. Every program is compiled to bytecode in both (a
  construct the bytecode compiler can't lower fails the build, naming the `kazi` and line; there
  is no syntax-tree artifact). `dev` (default) writes the native image where it can and otherwise
  leaves it to the runner to build at start-up; `release` requires the native image (a platform
  without a backend fails the build), writes a standalone executable and never uses the build
  cache. `embedded` is rejected as not implemented yet, and unknown names are rejected
- `--muda`: Print phase-latency timings (`kuchanganua` = compile, `kutoa` = emit) after a
  successful build, via `pata_cli::pipeline::performance::PerformanceMetrics`/`ScopedTimer`.
  Works on both the normal build path and the single-file cache-hit early-return path.

Success:
- parses `pata.toml` (or builds single file without manifest)
- compiles from `[chanzo].kuingia`
- emits a bytecode `.asb` artifact, plus a `.build.manifest` sidecar under the output directory
- compiles it ahead of time to native machine code with Asili's own backend (`nguvu`; no
  external compiler, assembler or linker) into `<name>.nguvu` beside the `.asb`, printing
  `msimbo asilia: <path>`. Where the image cannot be built (`dev`) it prints
  `picha ya msimbo asilia haikujengwa (...)` and the build still succeeds; when the image is
  missing or stale at run time, `tenda` compiles native code in memory instead (a platform with
  no backend cannot run the program). Leftover `<name>.so`/`<name>.ll` files from the retired
  LLVM backend are removed.
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
