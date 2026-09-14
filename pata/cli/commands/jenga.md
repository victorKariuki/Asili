# `pata jenga`

Purpose: build project sources into bytecode or target binary. Optionally display workspace information.

Flags:
- `--workspace-info`: Display workspace members from `Asili.toml`'s `[workspace]` table and exit
- `--tenda`: After building, execute `kuu` with any trailing arguments
- `--pato <path>`: Output directory (default `kilele/`)
- `--lengo <lengo>`: Build target (default `native`); overrides `pata.toml`'s `[jenga] lengo`
- `--namna <profile>`: Build profile (dev/release/embedded) — not yet functional

Success:
- parses `pata.toml` (or builds single file without manifest)
- compiles from `[chanzo].kuingia`
- emits `.asb` bytecode and `.build.manifest` sidecar under output directory

Failures:
- syntax/type/ownership diagnostics
- dependency resolution failures
- target backend not available
