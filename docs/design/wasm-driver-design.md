# Wasm driver design

The Wasm driver is `driver/wasm` (crate `asili-wasm`), a thin layer that runs Asili source in a
WebAssembly environment. It depends directly on `asili-lexer`, `asili-parser`, and
`asili-evaluator` (`driver/wasm/Cargo.toml:9-14`) — no separate bytecode/codegen step; a Wasm
build still tree-walks the same AST as the native CLI.

## Two targets, one crate

`driver/wasm/Cargo.toml:16-22` defines two Cargo features, each mapped straight through to the
same-named feature on `asili-evaluator`:

- **`wasm-browser`** — target `wasm32-unknown-unknown`. Pulls in `wasm-bindgen` and
  `serde-wasm-bindgen` (both `optional = true`, `Cargo.toml:13-14`) and enables
  `asili-evaluator/wasm-browser`, which routes stdout/stderr to `console.log`/`console.error`.
- **`wasm-wasi`** — target `wasm32-wasip1`. Enables `asili-evaluator/wasm-wasi` only; no extra
  dependency. There is no `wasi` crate anywhere in `driver/wasm/Cargo.toml` or
  `core/evaluator/Cargo.toml` — confirmed by reading both files in full. Rust's `std` already has
  first-class support for `wasm32-wasip1`, so `println!`/`eprintln!`/`std::fs`/stdin work
  unmodified; the feature exists only to stop the evaluator from special-casing `target_arch =
  "wasm32"` when the WASI target is what's actually running.

`core/evaluator/Cargo.toml:28-35` carries a matching comment: `wasm-browser` is "mutually
meaningful only on wasm32" (a no-op on native, since the platform module gates on `target_arch`
too), and `wasm-wasi` "just stops special-casing wasm32." `core/evaluator/Cargo.toml:19-23` also
adds a `wasm32`-only dependency on `getrandom` with the `js` feature — needed so `rand` (used
elsewhere in the evaluator) can call `crypto.getRandomValues` via wasm-bindgen's JS glue on
`wasm32-unknown-unknown`; a no-op for `wasm32-wasip1`, which has native WASI random support.

## Why the feature lives on `core/evaluator`, not just `driver/wasm`

`core/evaluator/src/platform.rs` is a small I/O shim (`write_stdout`/`write_stderr`/`read_stdin`)
that the builtins call into instead of using `println!`/`eprintln!`/`std::io::stdin` directly.
It exists because the same `asili-evaluator` crate has to build for two different `wasm32`
targets with fundamentally different I/O mechanisms — a Cargo feature on `driver/wasm` alone
can't change how `core/evaluator`'s own builtin functions behave, since features are additive
per-crate. So the gating has to be a feature on `core/evaluator` itself (`wasm-browser` /
`wasm-wasi`), which `driver/wasm` just forwards to via its own same-named features.

`platform.rs:1-10`'s own doc comment spells out three cases, confirmed by reading the full file:

1. **Non-wasm32, or wasm32 with `wasm-wasi`** (`platform.rs:14-39`) — std's `println!`/
   `eprintln!`/`stdin().read_line` work correctly as-is.
2. **wasm32 with `wasm-browser` and not `wasm-wasi`** (`platform.rs:41-67`) — `write_stdout`/
   `write_stderr` call `console.log`/`console.error` through a `#[wasm_bindgen] extern "C"` block
   (`platform.rs:42-52`). `read_stdin` returns `Err(EvalError::Panic("omba: stdin haipatikani
   kwenye kivinjari"))` — there is no stdin in a browser, so this is an explicit error, not a
   silent no-op.
3. **wasm32 with neither feature** (`platform.rs:69-78`) — the historical silent-no-op default:
   `write_stdout`/`write_stderr` do nothing, `read_stdin` returns an error. This keeps an
   unconfigured `wasm32-unknown-unknown` build compiling and running (just without real I/O) if
   someone builds `asili-evaluator` for wasm32 without picking either feature.

Other builtins gate the same three ways on the same `cfg` conditions: `core/evaluator/src/
builtins/majira.rs` (time), `builtins/mfumo.rs` and `builtins/runtime.rs` (system/env info), and
`builtins/neno.rs` gate on bare `target_arch = "wasm32"` without the wasi carve-out (confirmed by
grep across `core/evaluator/src/`).

## File I/O on wasm32: explicit errors, not silent no-ops

`core/evaluator/src/builtins/faili.rs` (`soma_faili`, `andika_faili`, `ongeza`, `vipo`, `futa`,
`ukubwa`) gates each builtin the same way: `#[cfg(any(not(target_arch = "wasm32"), feature =
"wasm-wasi"))]` uses real `std::fs`; `#[cfg(all(target_arch = "wasm32", not(feature =
"wasm-wasi")))]` — i.e. `wasm32-unknown-unknown`, with or without `wasm-browser` — returns an
explicit `Value::Tokeo(Err(...))` carrying a message like `"soma_faili: haipatikani kwenye
kivinjari"` ("not available in the browser"), rather than silently no-oping
(`faili.rs:11-24,25-39,40-57,66-79`). The two exceptions are `vipo` (exists-check) and `ukubwa`
(file size), which can't return a `Tokeo` given their existing signatures (`Ukweli`/`Namba`), so
they fall back to `false`/`0.0` on an unconfigured wasm32 build (`faili.rs:58-65,80-89`) — still
distinguishable from a real result, just not via the `Tokeo` error channel the other four use.

This matches `driver/wasm/README.md:10`'s and `platform.rs`'s framing that WASI
(`wasm-wasi`) gets real file I/O and browser (`wasm-browser` alone) does not — file access has no
browser equivalent, unlike stdout/stderr which map onto `console.log`/`console.error`.

Note: `driver/wasm/README.md:10` still describes `chapisha`/`majira`/`vigezo`/`pata_env` as using
"spec-compliant stubs" on wasm32 without mentioning the `wasm-wasi`/`wasm-browser` split added
since — that line predates the current three-way `cfg` scheme in `platform.rs` and is stale.

## Entry points

Two functions in `driver/wasm/src/lib.rs`, both plain Rust (`Result<(), String>`), independent of
any binding layer:

- **`run_source(source: &str) -> Result<(), String>`** (`lib.rs:15-20`) — tokenize, parse,
  `run_main(&module, vec![])`. Single-module only: any `leta` import is left unresolved and its
  exports just won't be found (no error at this layer).
- **`run_bundle(entry_name: &str, modules: &HashMap<String, String>) -> Result<(), String>`**
  (`lib.rs:27-66`) — multi-module. Looks up `entry_name` in `modules`, parses it, then does an
  in-memory worklist walk: for each unresolved `leta` target (from `ImportPath::Full` or
  `ImportPath::Selective`), look it up in `modules`, parse it, queue its own imports, repeat. An
  import with no matching entry in `modules` is silently skipped (`let Some(source) =
  modules.get(&name) else { continue }`, `lib.rs:48`) — not an error, same "unresolved import"
  behavior as `run_source`. Once every reachable module is parsed, it calls
  `asili_parser::merge_modules(&entrypoint, &parsed)` once against the full resolved map and runs
  the result.

### Comparison with the CLi's disk-based resolver

`run_bundle` is the filesystem-less counterpart to `pata/cli/src/pipeline/resolve.rs`'s
`resolve_all`/`find_module_file` — same job (turn `leta` imports into a merged, evaluable
`Module`), different source of truth:

| | `driver/wasm::run_bundle` | `pata/cli`'s `resolve_all` |
|---|---|---|
| Module source | in-memory `HashMap<String, String>` (name → source), supplied by the host | disk, via `find_module_file` (`resolve.rs:61-105`): path deps, vendored `.asili/packages/`, local `<root>/<name>.as`, `lib/`, stdlib `.asi` |
| Missing import | silently skipped (`lib.rs:48`) | reported as diagnostic `RES002` with full searched-path list |
| Cycle detection | none — a `while let Some(name) = pending.pop()` worklist that dedups via `parsed.contains_key`, so a cycle just terminates instead of being flagged | explicit, diagnostic `RES002`'s sibling `RES001` |
| Merge step | single call to `merge_modules` against the whole resolved map after the worklist drains | same `merge_modules` call, but reachable via `resolve.rs`'s `merge_for_eval` |

**They do share the actual merge logic.** `run_bundle` calls `asili_parser::merge_modules`
(`core/parser/src/module_merge.rs:15-77`) directly (`lib.rs:9,63`) — the same function
`pata/cli`'s `merge_for_eval` delegates to, as documented in
[package-manager-design.md](package-manager-design.md#how-patacli-delegates-to-pata_package).
`module_merge.rs:1-5`'s own doc comment confirms this is deliberate: "Shared by `pata/cli`'s
disk-based resolver ... and `driver/wasm`'s in-memory bundler, so the merge rules ... live in
exactly one place." Concretely this means: public functions/structs/traits (or the selective list
from `leta X::{a, b}`), every module-level `thabiti` constant regardless of visibility, and every
impl block from an imported module are merged identically whether the module text came from disk
or from an in-memory map — the same export/visibility bugs or fixes apply to both drivers, and
[package-manager-design.md](package-manager-design.md)'s bug write-ups (public constants,
struct/trait/impl merging) apply equally to `driver/wasm`.

## Host bindings

`driver/wasm/src/lib.rs:68-89`, gated `#[cfg(all(target_arch = "wasm32", feature =
"wasm-browser"))]`:

- `#[wasm_bindgen] pub fn run(source: &str) -> Result<(), JsValue>` — wraps `run_source`,
  converting the `String` error to `JsValue::from_str`.
- `#[wasm_bindgen(js_name = runBundle)] pub fn run_bundle_js(entry_name: &str, modules: JsValue)
  -> Result<(), JsValue>` — converts the incoming `JsValue` (a JS object/Map-like value) to
  `HashMap<String, String>` via `serde_wasm_bindgen::from_value`, then calls `run_bundle`.

For WASI, `driver/wasm/src/bin/asili_wasi.rs` is a standalone binary target: `asili_wasi
<faili.as>` reads the file from disk (real `std::fs`, since the binary itself runs under a WASI
host with filesystem access granted), calls `asili_wasm::run_source`, and maps `Ok`/`Err` to
`ExitCode::SUCCESS`/`FAILURE` with the error printed to stderr. It is single-module only — no
`run_bundle` binary/CLI wiring exists for WASI (multi-module WASI programs would need a way to
supply the `modules` map, which the binary doesn't currently expose, e.g. no directory-walk to
build one from a `src/` tree).

## Tests

`driver/wasm/src/lib.rs:91-127`, plain `#[cfg(test)]` unit tests (run natively, not compiled to
wasm32 — no `wasm-bindgen-test` harness present):

- **`run_source_minimal_kuu`** (`lib.rs:96-104`) — a `kuu` function with an empty body just
  containing `rejesha`; asserts `run_source` succeeds.
- **`run_bundle_resolves_multi_module_leta`** (`lib.rs:106-119`) — two in-memory modules
  (`kuu` importing `mathutil`, which declares `thabiti PI: Namba = 3.14`); the entry constructs a
  struct literal using the imported constant. Asserts `run_bundle` succeeds, i.e. that
  cross-module constant resolution works through the in-memory bundler.
- **`run_bundle_missing_entry_is_an_error`** (`lib.rs:121-126`) — calling `run_bundle("kuu", &
  HashMap::new())` (entry itself absent) returns `Err` containing `"kuu"` — this is the one case
  `run_bundle` does report as an error (the entry module, unlike a transitive import, must be
  present or there's nothing to run).

No test exercises the `wasm-browser`/`wasm-wasi` `#[wasm_bindgen]`/binary-target code paths
directly (they're feature-gated to actual wasm32 builds); `run_source`/`run_bundle` themselves are
target-agnostic Rust, so the unit tests above cover their logic without needing a wasm32 target or
JS host.

## Not yet implemented / known gaps

- No `run_bundle` equivalent for the WASI binary (`asili_wasi`) — single-file only.
- No cycle detection in `run_bundle`'s import walk (a self-importing module pair just terminates
  via the `parsed` dedup rather than being flagged, unlike the CLI's `RES001`).
- A missing import module is silently skipped in both `run_source` (implicitly, since it never
  resolves at all) and `run_bundle` (explicitly, `lib.rs:48`) — no equivalent of the CLI's
  `RES002` diagnostic with a searched-path list; a Wasm host gets no feedback about *why* a name
  didn't resolve.
- `#[sharti(lengo = "wasm")]` filtering (see [sharti-design.md](sharti-design.md)) is a
  `pata/cli` pipeline pass (`pata/cli/src/pipeline/sharti.rs`) — `driver/wasm` doesn't run it.
  Source handed to `run_source`/`run_bundle` directly (bypassing `pata jenga --lengo wasm`)
  still contains any `#[sharti(lengo = "native")]`-gated items; they simply won't be filtered
  out before evaluation. In practice this only matters if such an item is actually reachable from
  `kuu` and does something target-inappropriate — the item type-checks and evaluates fine either
  way, since the evaluator has no `#[sharti]` awareness of its own.
- `driver/wasm/README.md`'s I/O description (`README.md:10`) is stale relative to
  `platform.rs`'s current three-way `wasm-browser`/`wasm-wasi`/unconfigured split.

## Cross-references

- [package-manager-design.md](package-manager-design.md) — `module_merge.rs`'s merge rules, shared
  verbatim with this crate's `run_bundle`.
- [sharti-design.md](sharti-design.md) — the `#[sharti(lengo = "...")]` filter pass that runs in
  `pata/cli`'s pipeline but not in `driver/wasm`.
- [implementation-status.md](implementation-status.md#phase-ii--synthesis-lsp-wasm-tooling) — phase
  checklist entry this doc supersedes with detail.
- [docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md) — Phase II Wasm
  entry in the phase/feature map.
- [mwalimu-design.md](mwalimu-design.md) / [package-manager-design.md](package-manager-design.md)
  — sibling design docs this one follows in structure/style.
