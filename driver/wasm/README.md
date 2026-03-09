# Asili Wasm driver (Phase II)

Thin layer to run Asili in WebAssembly. Build with:

```bash
cargo build --target wasm32-unknown-unknown -p asili-wasm
```

- **Entry:** `asili_wasm::run_source(source: &str) -> Result<(), String>` — tokenize, parse, run `kuu` with empty args. Single-module only.
- **I/O:** Evaluator builtins (`chapisha`, `majira`, `vigezo`, `pata_env`) use spec-compliant stubs when built for `wasm32` (no std::env, no SystemTime, no println).
- **Host:** For JS, add `wasm-bindgen` and export `run_source`; for WASI, use the same binary in a WASI runtime (e.g. wasmtime).
