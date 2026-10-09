# Asili Wasm driver

Runs Asili in the browser (the playground, `examples/playground`). Build with:

```bash
cargo build --release --target wasm32-unknown-unknown -p asili-wasm --features wasm-browser
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/asili_wasm.wasm
```

- **Entry:** `run(source)` / `runBundle(entry, modules)` (`run_source` / `run_bundle` in Rust):
  tokenize, parse, lower to bytecode and run `kuu` as native code.
- **Native code:** `nguvu` compiles the program to a wasm module of its own and asks the page to
  instantiate it beside this one — the page must define
  `globalThis.asili_nguvu_load(bytes, count, memory, table)`, which grows `table` by `count`,
  instantiates `bytes` with `{ env: { memory, table, base } }` (`base`: the table's old length)
  and returns `base` (see `examples/playground/main.js`). The build exports a growable function
  table for this (`build.rs`).
- **I/O:** `chapisha`/`onyo`/`makosa` go to `console.log`/`console.error`; there is no stdin or
  file system in a browser.
- **Tests:** `tests/agree.sh` runs every example through this build under Node
  (`tests/run_node.mjs`) and checks it prints exactly what native code prints.
