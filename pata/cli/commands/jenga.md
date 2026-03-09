# `pata jenga`

Purpose: build project sources into bytecode or target binary.

Inputs:
- optional target profile (dev/release/embedded)
- optional output directory override

Success:
- parses `pata.toml`
- compiles from `[chanzo].kuingia`
- emits artifacts under `target/`

Failures:
- syntax/type/ownership diagnostics from Mwalimu
- dependency resolution failures
- target backend not available
