# Changelog

All notable changes to the Asili implementation (repository and tooling) are recorded here. For specification changes, see [docs/spec/CHANGELOG.md](docs/spec/CHANGELOG.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **`pata jenga --workspace-info`**: detect and display workspace member information from
  `Asili.toml`'s `[workspace]` table, via `find_workspace_root` → `pata_package::Workspace::open`
  (previously fully implemented in `pata_package` but called from no CLI command at all).
- **`pata jaribu --nyuzi-za-jaribio <n>`**: parallel test execution using a rayon work-stealing
  thread pool. Default behavior unchanged (sequential, `nyuzi == 1`); pass a thread count to run
  tests concurrently.
- **`pata jaribu --json`**: structured test-result output (`{"jumla", "sawa", "kosa",
  "majaribio"}`) instead of `[SAWA]`/`[KOSA]` lines, for tooling to consume.
- **`pata njozi --kiasi` / `--maktaba` / `--workspace`**: template variants for binary vs. library
  project scaffolding, and a multi-member workspace layout (`Asili.toml` + per-member
  `pata.toml`s).
- **`pata njozi`-generated projects now include `.github/workflows/ci.yml`** (build/test/lint
  stub), matching this repo's own CI workflow shape.
- **`pata ondoa <lib>`**: the missing counterpart to `pata ongeza` — removes a dependency from
  `pata.toml` and regenerates `pata.lock`.
- **Adversarial test coverage for existing `pata-lint` rules** (`LINT001`–`LINT003`, `LINT101`,
  `LINT201`–`LINT203`): each now has real false-positive/false-negative test cases, not just
  happy-path coverage.
- **LSP inlay hints, for real**: `pata/lsp/src/inlay_hints.rs::compute_inlay_hints` now runs the
  same `SemanticAnalyzer` scope-resolution walk semantic-tokens uses (a new
  `inlay_type_hints()` accessor collects `(line, column, ValueType)` for every un-annotated
  `weka`/`thabiti` while `scan_stmt` is already inferring types for scope binding), and renders
  a real `InlayHintLabel::String(": <Aina>")`. Replaces the previous version, which built every
  hint with an empty `LabelParts(vec![])` label (rendered as nothing) from a naive per-line text
  scan that checked for a `kitu` keyword this language's lexer doesn't have (declarations use
  `weka`).
- **LSP code actions, for real**: the `code_action` handler's LINT202 fix now matches the
  lint rule's real Swahili message (`"kazi '<name>' haina maelezo..."`) instead of a
  `"Function '<name>'"` prefix that never matched anything, so the fix silently never fired even
  though the surrounding plumbing (`edit: Some(...)`) was correct. A second fix for LINT203
  ("Ondoa leta isiyotumika") now deletes the flagged import line for real. Both are implemented
  as a pure `actions::action_for_diagnostic(diag, uri) -> Option<CodeAction>` function, unit
  tested directly against realistic diagnostic messages rather than only reachable through the
  full `tower_lsp` server.
- **`LINT203` ("unused leta") now does real per-name detection**: for each `leta
  modname::{a, b}` selective import, flags any name that never appears as a word elsewhere in
  the file — replacing the previous check, which only ever fired when a file had imports *and
  zero functions* (`module.imports.len() > 0 && module.functions.is_empty()`), a condition that
  had nothing to do with whether an import was actually used. Wildcard `leta modname` imports
  aren't checked (no explicit names to search for without full usage resolution).
- **`pata_lint::config::LintConfig` is wired in**: `pata-lint`'s CLI and the LSP's diagnostics
  pipeline (`pata/lsp/src/diagnostics.rs::run_lex_parse`) both now load a project's
  `[lint.rules]` `pata.toml` table via a new `LintConfig::find_and_load` (walks upward from the
  file/directory being linted to the nearest ancestor `pata.toml` — `pata-lint` has no
  dependency on `pata-cli`'s own project-root walk, so this is a small independent one) and
  apply it through a new `lint_source_with_config`: `severity = "ignore"` genuinely suppresses a
  rule's diagnostics, and `LINT101`'s `options.line_limit` genuinely overrides its default
  50-statement threshold. `lint_source`/`pata-lint`'s default CLI behavior is unchanged when no
  `pata.toml`/`[lint.rules]` is present.
- **`pata_fmt::config::FormatterConfig`'s `indent_style`/`indent_width` are wired in**:
  `pata-fmt`'s `main.rs` now loads `[fmt]` via a new `FormatterConfig::find_and_load` (same
  upward-walk shape as `LintConfig`'s) and threads a real indent unit into a new
  `canonical_format_with_indent`, replacing the printer's previously-hardcoded 4-space indent.
  `line_width` (line-wrap) is still not applied — `Printer` has no line-length tracking or wrap
  points at all, so this is honestly out of scope until that's built, not silently claimed done.

### Fixed

- **Lint test suite**: corrected `LINT101` boundary test (50 statements exactly should not flag;
  flag only when > 50). Tests now confirm the exact threshold behavior.
- **`pata-fmt` tab-indent correctness bug**: `Printer::emit_token`'s `needs_space_before` check
  only tested `self.out.ends_with(' ')` to detect "already at a fresh indent, don't add another
  leading space" — true for the old hardcoded 4-space indent (which does end in `' '`) but false
  right after a tab, so configuring `indent_style = "tabs"` produced a spurious extra space after
  every tab (`"\t weka x = 1"` instead of `"\tweka x = 1"`). Caught by a real test
  (`tab_indent_is_applied`) added while wiring `FormatterConfig`, not by inspection — this bug
  was latent in the printer before the config was ever wired to reach it.

### Added, but not yet usable (real code, real unit tests, but not actually doing anything a user
### would notice yet — either not called from any command, or called but producing empty/inert
### output. Verified by reading the actual call sites and function bodies/return values, not by
### re-reading commit messages)

- **`rules/logic.rs`'s `check_logic_errors`**: wired into `pata_lint::lint_source_with_config`
  (gated on `LINT301`'s `is_enabled`, same as every other rule), but the function itself is
  `_module: &Module) -> Vec<Diagnostic> { Vec::new() }` — a genuine no-op, per its own doc
  comment ("placeholder for future depth"). Being *called* is not the same as doing anything.
- **`pata_package::LocalRegistry`/`PackageMetadata`/`RegistryEntry`** (`pata/package/src/
  registry.rs`) and **`pata_package::VersionConstraint`** (`constraints.rs`, real semver
  constraint parsing): both exist with real unit tests, neither has any real caller in
  `pata/cli` — `Resolver::resolve` still locks whatever version string is given verbatim.
- **`pata_cli::pipeline::stability::check_type_stability`** (`pata/cli/src/pipeline/stability.rs`):
  git-tag-baseline type-stability checking. `pata thibitisha` has no `--baseline` flag and never
  calls this — `thibitisha.rs`'s own comment still says "Not implemented: type-stability."
- **`pata_cli::pipeline::coverage::CoverageMetrics`** (`pata/cli/src/pipeline/coverage.rs`):
  function-execution coverage tracking. `pata jaribu` has no `--chanjo`/coverage flag and never
  calls this.
- **`pata_cli::pipeline::performance::PerformanceMetrics`/`ScopedTimer`**
  (`pata/cli/src/pipeline/performance.rs`): phase-latency/SLO tracking. Not called from any
  command.
- **`pata_fmt::config::FormatterConfig::line_width`**: parsed and unit-tested, but genuinely
  inert — see the `FormatterConfig` entry above under Added.
- **`pata-lsp`'s own `format.rs::format_document`** is a *different, weaker* formatter than
  `pata-fmt`'s `canonical_format` (crude line-by-line brace/comma whitespace normalization vs. a
  real token-stream printer with reindentation, generic-bracket handling, and comment
  preservation), despite a doc comment claiming it's "inlined from
  pata/cli/src/pipeline/format.rs::canonical_format". `pata-fmt` is a `[[bin]]`-only crate with
  no `[lib]` target today, so `pata-lsp` (which can't depend on `pata-cli` — a separate,
  pre-existing circular-dependency constraint, see `docs/design/pata-production-readiness.md`'s
  "Full build-out" section, item 0) has nothing real to depend on yet. Format-on-save in the
  editor is therefore still running the weaker algorithm, and doesn't see the `FormatterConfig`
  wiring added this cycle either. Fixing this for real needs the `pata-core`/shared-lib
  extraction already flagged there, not a further inline copy.

### Not started

Real gaps, no commits addressing them yet: per-test timeout for `pata jaribu`; `#[kabla]`/
`#[baada]` setup/teardown fixtures; a shared `pata-core` crate (resolver extraction, and now also
the `pata-fmt`/`pata-lsp` formatter-duplication fix above); a DAP (Debug Adapter Protocol)
server; `pata-fmt` line-width/wrap-point support. Tracked as GitHub issues (see the `Asili
Feature release` project board).

## [0.5.0] — pata-cli; asili-evaluator, pata-package at patch bumps

### Added

- **`pata ondoa <lib>`**: the missing counterpart to `pata ongeza` — removes a dependency from
  `pata.toml` and regenerates `pata.lock`.
- **`pata_package::fetch_git`/`hash_dir`** (`pata/package/src/fetch.rs`): real git-clone-based
  dependency fetching (via `git2`) and a deterministic SHA-256 content hash over a fetched tree,
  replacing the `sha256("{name}@{version}")` name-string placeholder the old checksum used. Not
  yet wired into `pata ongeza` itself — `pata ongeza <lib> <version>` still only writes to
  `pata.toml`/`pata.lock` without fetching — so this ships as library-internal groundwork this
  release, not a user-facing fix yet (tracked separately).

### Fixed

- **`rand` 0.8→0.10 API migration** (`core/evaluator/src/builtins/hisabati.rs`): `rand::thread_rng()`
  was removed upstream in favor of `rand::rng()`, and `Rng::gen()` moved to a new `RngExt::random()`
  trait. `nasibu`/`nasibu_chini`'s Asili-level behavior is unchanged — internal-only.
- **`pulldown-cmark` 0.9→0.13 API migration** (`pata/cli/src/commands/repl.rs`): `Tag::Heading`
  became a struct variant, and `Event::End` now carries the lighter `TagEnd` enum instead of the
  full `Tag`. Fixes the `?topic` REPL markdown renderer, which stopped compiling under the bump;
  verified live against the real REPL, rendering unchanged.
- **`sha2` 0.10→0.11 API migration** (`pata/package/src/lock.rs`, `resolver.rs`): the digest
  output type lost its direct `LowerHex` impl; replaced `format!("{:x}", ...)` with an explicit
  byte-to-hex map producing the same checksums as before.
- **`.github/dependabot.yml`** was still GitHub's unfilled default template
  (`package-ecosystem: ""`, invalid) despite two Dependabot PRs having already merged into `main`
  via zero-config security updates. Now targets `cargo`/`npm`/`github-actions` explicitly, each
  with `target-branch: develop`, so future Dependabot PRs open against `develop` instead of `main`.
- **VS Code extension packaging**: `vsce package` silently produced a broken `.vsix` missing its
  own entrypoint in this project's dev environment, because npm 11's built-in secret-redaction
  feature mangles any path containing a UUID-shaped segment in `npm list`'s `--parseable` output —
  exactly what `vsce` parses to find `node_modules` to bundle. Fixed by having `esbuild` inline
  the extension's runtime dependency (`vscode-languageclient` and its own transitive deps) into a
  single `out/extension.js` at build time instead, so `vsce` has nothing left to detect — the
  currently-recommended approach for VS Code extensions generally, not just a workaround. Also
  fixes `engines.vscode` (was trailing behind an `@types/vscode` bump) and adds a `license` field
  and `LICENSE` file so `vsce package` stops warning.
- **`extensions/vscode/tsconfig.json`**: `vscode-languageclient` 9→10 ships type declarations
  requiring `moduleResolution: "node16"` (TypeScript's default resolution couldn't locate
  `vscode-languageclient/node` despite the `.d.ts` file existing on disk); `module` had to move to
  `"node16"` alongside it. Emit stays CommonJS in practice (`package.json` has no
  `"type": "module"`), verified by inspecting the compiled output.

### Changed

- `Makefile`'s `install-ext` target (and the matching "Package + install VS Code extension" /
  "Build + install all" VS Code tasks) now actually build, package, and install the VS Code
  extension end to end (`npm install && vsce package --no-dependencies && code
  --install-extension`), instead of only copying the `pata-lsp` binary into
  `extensions/vscode/bin/` and calling it done.
- `extensions/vscode/node_modules/` (564 files, committed since the repo's first commit before
  `.gitignore`'s `node_modules/` rule existed) is no longer tracked — reproducible via `npm
  install`, and nothing in CI/the Makefile depended on it being pre-populated in git.
- Two new project-level rules, each with a matching `.claude/skills/` entry: never add AI
  attribution (`Co-Authored-By: Claude ...`) to commits or PR descriptions (five pre-existing
  trailers had to be removed via a `git filter-repo` rewrite + force-push to `main`/`develop`);
  and keep the project's two GitHub Project boards (`Asili bug tracker`, `Asili Feature release`)
  in sync with real work as it starts/ships, not just filed-and-forgotten.

## [0.4.0] — core/evaluator, core/lexer, core/parser, pata-cli, pata-fmt, pata-lsp

### Fixed

- **`pata nadhifu`**: replaced the naive `String::replace`-based formatter with a real
  token-stream pretty-printer. Fixes the exact bug class the old implementation had — string/char
  literals containing `{`/`}`/`,` were previously mangled by layout rules meant for real syntax.
  Comments (`#`/`//`) are now preserved instead of silently discarded (`asili-lexer` gained an
  additive `tokenize_with_trivia()`, leaving `tokenize()` and every existing consumer unchanged).
  Blank lines between top-level declarations are preserved as exactly one blank line. New
  `docs/design/nadhifu-formatter-design.md`.

### Added

- **`Value` <-> JSON codec**: `kwa_json(thamani) -> Tokeo<Neno, Neno>` / `kutoka_json(neno) ->
  Tokeo<Kamusi<Neno, Unknown>, Neno>`, exposed from the existing `mfumo` module. Rejects resource
  handles and concurrency primitives (no JSON representation) with a clear error; guards against
  unbounded recursion on adversarial input. See `docs/design/json-codec-design.md`.
- **Structured `EvalError`**: new `EvalError::Coded { kind: ErrorKind, message }` variant
  (`ErrorKind`: `BadInput`/`NotFound`/`Conflict`/`Internal`/`Unavailable`), additive alongside the
  six pre-existing variants. A `From<&EvalError> for ErrorKind` fallback maps every pre-existing
  variant to `Internal`. Lays the groundwork for a future HTTP layer to map errors to status
  codes without core-evaluator code needing to know about HTTP at all.
- **`njia_na_kikomo(kikomo: Namba)`**: a bounded-channel constructor alongside the existing
  unbounded `njia()` — `.tuma()` blocks once `kikomo` unread items are buffered, instead of
  growing memory without limit under a producer faster than its consumer.
- **`kasha_gc_dhaifu(kgc) -> Kasha_GC_Dhaifu<T>`** (downgrade) / **`.imarisha() ->
  Chaguo<Kasha_GC<T>>`** (upgrade): a weak-reference escape hatch for `Kasha_GC<T>`, which has no
  cycle collector — holding a `Kasha_GC_Dhaifu<T>` in one side of a reference cycle lets the
  strong count still reach zero.
- **`mkondo_sikiliza(anwani) -> Tokeo<MkondoSikilizaji, Neno>`** and **`mkondo_tumikia(sikilizaji,
  kazi_jina, idadi_ya_nyuzi) -> Tokeo<Tupu, Neno>`**: a TCP listening socket and a bounded
  worker-thread pool to serve it. `mkondo_tumikia` blocks forever, spawning a fixed number of
  long-lived worker threads that each independently accept connections and invoke
  `kazi_jina(mkondo: Mkondo) -> Tupu`, which owns the connection via the same `.soma()`/
  `.andika()`/`.funga()` methods the existing client-side `Mkondo` handle exposes. Every accepted
  connection gets a fixed read/write timeout; the pool size is itself the concurrent-connection
  cap. Extends the existing 1:1-OS-thread model rather than introducing an async runtime — see
  `docs/design/http-server-design.md` for the full bounded-pool-over-async rationale. HTTP/1.1
  framing is explicitly out of scope for this pass. New `examples/mkondo_server/`; new
  `docs/design/json-codec-design.md` and `docs/design/http-server-design.md`.
- **TLS**: `tls_sanidi(cheti_njia, ufunguo_njia) -> Tokeo<TlsUsanidi, Neno>` loads a PEM
  certificate/key pair (never panics on a bad file/malformed PEM/mismatched key — always
  `Tokeo(Kosa(...))`); `mkondo_tumikia` gained a 4th, optional `tls: Chaguo<TlsUsanidi>` parameter
  to serve TLS instead of plaintext, via `rustls` (pure Rust — no system OpenSSL dependency,
  keeping the WASM/cross-compile story clean; `rustls`/`rustls-pemfile` are scoped to non-`wasm32`
  targets only). `MkondoHandle` became `Option<MkondoStream>` (an enum over plain/TLS streams)
  so `.soma()`/`.andika()`/`.funga()` needed zero changes — TLS is fully transparent to
  `kazi_jina`. A handshake failure drops only that one connection, never the whole worker thread.
  New `docs/design/tls-design.md`.
- **HTTP/1.1 framing**: `mkondo_tumikia_http(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls) ->
  Tokeo<Tupu, Neno>` — a new, separate entry point alongside the raw-bytes `mkondo_tumikia` (not
  a mode flag, since the two `kazi_jina` contracts are incompatible: `kazi_jina(ombi: OmbiHttp)
  -> JibuHttp` here, vs. `kazi_jina(mkondo: Mkondo) -> Tupu` for raw bytes). Parses one
  `Content-Length`-bodied HTTP/1.1 (or 1.0) request per cycle via `httparse` (now a direct
  dependency), calls `kazi_jina`, writes a real response, and loops for the next request on the
  same connection under HTTP/1.1 keep-alive — closing only on `Connection: close`, a parse error
  (`400`), or disconnect. `Transfer-Encoding: chunked` returns `501` (not supported); no
  pipelining; no `Expect: 100-continue`; no HTTP/2. New `Mkondo` method
  `.soma_bailisi(kikomo: Namba) -> Tokeo<Neno, Neno>` (one bounded, non-EOF-seeking read) — the
  prerequisite this needed, since `.soma()`'s read-to-EOF semantics can't support keep-alive.
  `OmbiHttp`/`JibuHttp` are plain `Value::Struct`s, matched by field shape only (same pattern the
  JSON codec already uses). New `examples/http_server/`; new
  `docs/design/http-framing-design.md`.
- **`pata thibitisha` gained trait-completeness and FFI-safety checks**: every `sifa` reachable
  from the project (declared locally or via a `leta`-ed dependency's `.asi` stub) must now have
  at least one implementation somewhere in the project, not just internal consistency for impls
  that already name a trait (which `SEM105` already covered). `#[kiunganishi]`-tagged function
  signatures are now checked against the set of types that can safely cross a C ABI boundary
  (numeric/`Ukweli`/`Herufi`/`Tupu`/`Anuani` only — no heap-owning, GC'd, or generic-container
  types), rejecting the rest with a clear error rather than allowing a signature that could never
  be ABI-compatible. `.asi` interface stubs can now declare `sifa { }` blocks, parsed into real
  `TraitDecl`s so the same completeness check covers trait requirements that arrive via a
  dependency, not just ones declared in-project.

## [0.3.0]

### Added

- **`pata thibitisha --kiwango-cha-jaribio <0-100>`**: optional test-coverage-ratio gate (percent
  of `#[jaribio]` test functions relative to public `kazi`). Not enforced unless the flag is
  passed, so existing projects aren't broken by default.
- **`Faili`/`Mkondo`/`Kumbukumbu<T>` resource-handle types** (previously spec'd but entirely
  unimplemented — see `docs/design/faili-mkondo-design.md`). `faili_fungua(njia, hali)` (via the
  existing `leta faili`) and `mkondo_unganisha(anwani)` (via `leta mfumo`) return handles with
  `.soma()`/`.andika(data)`/`.funga()` methods; `kumbukumbu_unda(v)` (always in scope, no `leta`
  needed) wraps a value in a heap box with `.pata()`. `Faili`/`Mkondo` close their underlying OS
  handle automatically on scope exit — not just on explicit `tupa`/`.funga()` — via a real Rust
  `Drop` impl on the handle's wrapper type, since `Env::pop_scope` (ordinary block/function exit)
  bypasses `Env::drop` entirely and would otherwise leak the handle. New
  `examples/faili/`; new `docs/repl/{sw,en}/faili.md`.
- **Sifa (traits) now carry real method signatures and a completeness check**, instead of being
  nominal-only (a `TraitDecl`'s body used to be discarded entirely at parse time). Every
  `shughuli ya Target kwa Trait`/`shughuli ya Target: Trait` is checked against its trait's required methods
  (name/params/return type, `Self` resolved to the implementing type); a gap is a new
  diagnostic, `SEM105`. `Inasomeka`/`Inandikika` are now real built-in traits (seeded the same
  way `Chaguo`/`Tokeo` are, not via an `.asi` file — see
  `docs/design/sifa-traits-design.md` for why). No trait-object/dyn-dispatch — deliberately out
  of scope. New `examples/sifa/`; new `docs/repl/{sw,en}/sifa.md`.
- **`Seti<T>` (set)**: `seti(v1, v2, ...)`/`seti_tupu()` constructors, `.ongeza(v)`,
  `.ondoa(v) -> Ukweli`, `.ina(v) -> Ukweli`, `.urefu()`, `.orodha() -> Orodha<T>`, `.clona()`.
  Always in scope via `msingi`. `HashSet`-backed, so `.orodha()`'s iteration order is
  unspecified. Previously type-recognized (`ValueType::Seti` parsed and displayed correctly)
  but had zero runtime backing — see `docs/design/data-shapes-design.md`. New `examples/seti/`;
  new `docs/repl/{sw,en}/seti.md`.
- **`Namba_Kuu` (arbitrary-precision integer) / `Namba_Sahihi` (arbitrary-precision decimal)**:
  `num-bigint`/`bigdecimal`-backed. `namba_kuu_kutoka(neno)`/`namba_sahihi_kutoka(neno)` (via
  `leta hisabati`) parse a decimal string; `namba kama Namba_Kuu`/`kama Namba_Sahihi` widens
  infallibly from `Namba`, the reverse narrows fallibly (`Chaguo<Namba>`). Mixed arithmetic with
  `Namba` auto-widens; two `Namba_Kuu` operands stay in exact integer arithmetic (not silently
  promoted to decimal). No literal syntax by design — construction is exclusively through the
  constructors/casts above. Previously type-recognized only, zero runtime backing — see
  `docs/design/data-shapes-design.md`. New `examples/namba_kuu/`; new
  `docs/repl/{sw,en}/namba-kuu-sahihi.md`.
- **`Neno.herufi_kwa(i) -> Chaguo<Herufi>`**: character at grapheme position `i` (the same
  counting convention `.urefu()` already uses), `Hamna` if out of range. The narrow, real gap in
  string-manipulation coverage that
  `docs/design/phase3-self-hosting-borrow-checker-design.md`'s Phase III groundwork review
  identified — `.kata()`'s byte-range slicing can split a multi-byte character if the boundary
  is chosen wrong; this is the correct way to inspect one character at a time.
- **Decided the borrow-checker lifetime-inference strategy** (region-based inference with a
  narrow, named-region annotation surface for the two "complex structural boundary" cases
  `docs/spec/04-type-system.md` already commits to — `Rejeo`/`Rejeo_Tenda` stored in `umbo`
  fields, and function returns whose reference lifetime isn't inferable from a single parameter)
  — a decision, not an implementation; unblocks `Mfululizo<T>` and the real-reference-semantics
  work tracked in `docs/design/phase3-self-hosting-borrow-checker-design.md`, neither of which
  is implemented yet.
- **Concurrency: `tenda`/`njia`/`fungo`** (via `leta sambamba`), previously a complete
  non-functional stub under mismatched names (`anza_mwendo`/`subiri_mwendo`, matching nothing in
  the spec). 1:1 OS-thread model — `tenda(kazi_jina, hoja...)` spawns a real `std::thread`
  running the named module-level `kazi`, returning a handle; `subiri_tenda(id)` joins it,
  reporting only `Tokeo<Tupu, Neno>` (success/panic) — a spawned function's actual return value
  does not cross back this way, by design; communicate results via `njia` instead. `njia()`
  creates a channel (`.tuma()`/`.pokea()`); `fungo(v)` wraps a value behind a mutex
  (`.pata()`/`.weka()` lock-act-unlock atomically; `.funga()`/`.fungua()` are separate explicit
  lock/unlock for holding across several operations, matching the spec's literal naming).
  `Kasha_GC<T>`/`Faili`/`Mkondo` (or anything containing one) are rejected from crossing the
  thread boundary at all, checked recursively. Required a new `SendValue` type
  (`core/evaluator/src/value/mod.rs`) — a structural, compiler-checked `Send`-safe mirror of
  `Value` — after `unsafe impl Send` was considered and rejected on soundness/maintenance
  grounds; and the `parking_lot`/`lock_api` crates for `fungo`'s real manual lock/unlock (a
  `std::sync::Mutex` guard can't span two separate Asili-level calls). See
  `docs/design/concurrency-design.md` for the full architecture. New `examples/sambamba/`; new
  `docs/repl/{sw,en}/sambamba.md`.

### Fixed

- **`shughuli ya Target kwa Trait { ... }` (the `kwa`-keyword trait-impl syntax) never actually
  dispatched its methods** — `parse_impl_decl` had `target`/`trait_name` swapped for this syntax
  specifically, so `imp.target` held the *trait* name instead of the struct name and nothing
  method dispatch looks at ever matched. The `:`-colon syntax (`shughuli ya Target: Trait { }`)
  was always correct; only `kwa` was broken. This was previously documented as a known bug in
  `docs/language/07-mfumo-wa-aina.md` — found while building the Sifa completeness checker
  above, fixed, and now covered by regression tests (`core/evaluator/tests/traits.rs`).
- `pata thibitisha`'s public-doc-coverage check was flagging the newly-seeded `Inasomeka`/
  `Inandikika` traits as "missing documentation," since they have no real source location a doc
  comment could ever precede. Fixed to skip language-seeded declarations (recognized by their
  synthetic `line: 0`).
- **`mfumo::sikiliza_ishara`/`rejesha_ishara` return-type contract violation on non-Unix
  platforms.** The declared type contract said these return `Tupu`, but the non-Unix
  (`#[cfg(not(unix))]`) code path actually returned `Tokeo(Kosa(...))` — a real type mismatch
  between what the semantic analyzer promised and what the evaluator produced at runtime on
  Windows. Fixed by making both platforms consistent: both now return `Tokeo<Tupu, Neno>`, with
  the Unix success path wrapped in `Tokeo(Sawa(Tupu))` and the type contract in
  `core/parser/src/builtins.rs` updated to match.
- Deleted a stale TODO comment in `core/evaluator/src/signal.rs` claiming signal dispatch is
  "registered but never polled" — this was already fixed (confirmed: `eval/stmt.rs` and
  `eval/mod.rs` both call `signal::take_pending()` on every statement); the comment just never
  got removed after the fix landed. The real remaining signal-handling gap (silent no-op on
  non-Unix, now fixed above) is the only thing that TODO should have described.

### Changed

- `core/parser/src/semantic/analyzer.rs`'s `is_copy_type` TODO comment corrected — it claimed
  `Herufi`/`Tupu` still needed to be added as Copy types, but they already were; the comment was
  stale, not the logic. Reworded to describe the actual remaining gap (no runtime move-semantics
  enforcement).

## [0.2.1]

### Added

- **Bilingual REPL help docs**: `docs/repl/` is now split into `docs/repl/sw/` (Swahili, default) and `docs/repl/en/` (English) topic trees. A new REPL meta-command, `?lugha en` / `?lugha sw`, switches which tree `?topic` reads from for the rest of the session.

### Fixed

- **`pata/runner` (the standalone `.asb` deployment binary) was broken**: it still looked for the `.build.manifest` key `artifact=`, which was renamed to `kilele=` during the CLI's earlier Swahili-only rename — so it could never locate a build artifact from a current manifest. Fixed to read `kilele=`, matching `pata tenda`.
- Several genuine Swahili grammar/semantic-accuracy bugs found during a full pass over user-facing diagnostics and REPL docs, not just leftover English (`SEM023`'s and `SEM096`'s message-wording fixes are recorded under `[0.2.0]` below since they land in the same commit as the rest of the semantic-analyzer work):
  - `pata/cli/src/commands/thibitisha.rs`: `"umbo ya umma"` had the wrong noun-class agreement (umbo takes *la*, not *ya*); corrected to `"umbo la umma"`, matching the same agreement already used correctly elsewhere in `pata-lint`. `"nadhifu check imefeli"` (mixed English/slang) reworded to plain Swahili.
  - `pata/cli/src/commands/nadhifu.rs`: a cryptic `"tolea njia moja tu"` error now names the actual extra argument.
  - `pata/package/src/manifest.rs` and `workspace.rs`: `"Mtaa"` ("neighborhood/street") was used to mean "workspace member" — a real semantic mismatch, not just a style issue; renamed to `"Mwanachama"` to match the crate's own `members()`/"wanachama" terminology.
  - `pata/package/src/lock.rs`: an untranslated `"...lock file..."` fragment renamed to name the actual file, `pata.lock`.
  - `pata/lsp/src/symbols.rs`: the in-editor "▶ Run Test" CodeLens title (genuinely user-visible, unlike this file's internal doc comments) translated to "▶ Endesha Jaribio".
  - `pata/cli/src/commands/jenga.rs` and `tenda.rs`: usage/help text still referenced the old `target/` build directory after the `kilele/` rename; updated.
  - `docs/repl/`: a `linganisha`-with-bare-value example claimed an output (`Neno("mbili")`) the REPL never actually produces for a statement-form `linganisha` (verified live); corrected to use `chapisha` inside each arm. A stale `?linganisha` "topic" reference that isn't a real REPL command was reworded to a plain doc pointer. `neno.md`'s `.clona()` row had a non-example fragment in its Mfano column instead of code.
  - `docs/howto/06-use-repl.md`: claimed `thabiti` cannot be used at the REPL prompt — it is in fact the one documented exception that works; corrected, and its now-broken links into the old flat `docs/repl/*.md` layout updated for the new `sw/`/`en/` split.
- Completed the Swahili-wording sweep's last gaps: full translation of `docs/repl/{topic}.md`'s explanatory prose (previously Swahili headings over largely English body text, most notably `hisabati.md` and `kazi.md`), and translated section headings across all REPL topic docs.

## [0.2.0]

### Added

- **Msingi (prelude):** Constructors `tokeo(val)`, `kosa(ujumbe)`, `chaguo(val)`, `kamusi()`; constant `TUPU`. Type methods: `Chaguo` — `angu`, `ni_tupu`, `ni_po`, `hakikisha`; `Tokeo` — `ni_kosa`, `kosa`, `angu`; `Kamusi` — `funguo`, `vipo`.
- **Stdlib modules:** `runtime`, `syscall`, `kiungo`, `sambamba` (builtins and export tables); mfumo signals `sikiliza_ishara`, `rejesha_ishara` with eval-loop dispatch; `Anuani` type and signal state for Unix.
- **Prelude constant:** `TUPU` seeded in runtime and declared in msingi.
- **`#[sharti(target = "...")]` conditional compilation:** predicate parser and a new pipeline pass (`pata/cli/src/pipeline/sharti.rs`) that filters top-level enums/structs/traits/impls/functions by active build target before semantic analysis. `target = "a" | "b"` (OR-only) is supported; unrecognized predicate keys are a compile error.
- **Build target selection:** `pata.toml` `[jenga] lengo = "..."` manifest key and `pata jenga --target`/`--lengo` CLI flag (flag overrides manifest; defaults to `"native"`). Build caches (`.asb-cache`, project input hash) are target-aware so switching targets forces a rebuild instead of reusing a stale artifact.
- **`pata-package` wired into `pata-cli`:** dependency resolution and `pata.lock` now delegate to `pata_package::Resolver`/`LockFile` (real per-dependency SHA-256 checksums) instead of a hand-rolled, non-cryptographic hash. Version dependencies now resolve against a vendored `.asili/packages/<name>` cache in addition to path dependencies.
- **`examples/cross_package/`:** a two-package example (`app` + `mathutil`) demonstrating a `[tegemezi]` path dependency, `leta`-importing a struct and a public constant across packages.
- **Wasm driver, real I/O:** `driver/wasm` gains two Cargo features — `wasm-browser` (wasm-bindgen `run`/`runBundle` entry points routing `chapisha`/`onyo`/`makosa` to `console.log`/`console.error`) and `wasm-wasi` (stops special-casing wasm32 for I/O builtins entirely — Rust's std already has native WASI support for println!/stdin/fs/env/time/process::exit, confirmed by cross-compiling to `wasm32-wasip1` and `wasm32-unknown-unknown` and running both under Node's WASI/wasm-bindgen loaders). Neither feature needs the `wasi` crate.
- **Multi-module `leta` in Wasm:** `driver/wasm::run_bundle(entry_name, modules)` resolves imports from an in-memory `HashMap<String, String>` (no filesystem), sharing the same merge logic as `pata/cli`'s disk-based resolver via the new `asili_parser::merge_modules` (factored out of `pata/cli/src/pipeline/resolve.rs`'s `merge_for_eval`).
- `driver/wasm/src/bin/asili_wasi.rs`: a standalone binary entry point for running a `.as` file under `wasmtime`/Node WASI/etc.
- **`Kasha_GC<T>` (managed memory, opt-in `leta kasha_gc`):** a reference-counted shared wrapper layered on top of the default ownership model (not a replacement, and not a cycle-collecting GC — a self-referential `Kasha_GC<T>` leaks). New `Value::KashaGC(Rc<RefCell<Value>>)` runtime variant and matching `ValueType::KashaGC` static type (parsed as `Kasha_GC<T>`, formatted the same in hover/diagnostics). Sharing and refcounting need no special evaluator plumbing beyond the variant itself: `Value::clone()` on it is `Rc::clone` (cheap, shares), and `tupa`/scope-exit already drops the `Value` normally, decrementing the strong count via ordinary Rust ownership. Methods: `kasha_gc_unda(v)` (construct), `.pata()` (read a snapshot), `.weka(v)` (mutate in place), `.shirikisha()` (explicit share — like Rust's `Rc::clone`; plain `weka b = a` still moves `a`, unchanged from every other type), `.idadi()` (live handle count). `RefCell` borrow conflicts return a clean `EvalError::Panic`, never a raw Rust `BorrowError`/abort. Documented in `docs/language/06-moduli.md`; example at `examples/kasha_gc/`.

### Changed

- **Msingi:** Removed procedural helpers `chaguo_au_namba` and `ni_hamna` in favor of Chaguo methods (`.angu()`, `.ni_tupu()`, etc.). Examples (astar, data_structures) updated to use method calls.
- **Prelude:** Semantic and pipeline use prelude-only (msingi) for initial scope; duplicate-import checks attribute prelude names to msingi.
- **I/O and time:** `chapisha`, `paparika`, `omba` moved to `matumizi`; time helpers and `sasa` in `majira`; file ops in `faili`. Explicit `leta matumizi` (or relevant module) required for I/O.
- **Semantic analysis of `leta` imports:** the fixed builtin-module whitelist (`SEM007`) now also accepts any module name the project resolver already confirmed exists (user modules, path/vendored dependencies) — previously *any* non-builtin `leta` target was rejected outright, so cross-module/cross-package imports never actually worked despite the resolver supporting them. `pata/cli`'s final semantic pass now runs against the merged module (imported structs/traits/impls included) instead of the bare entrypoint, so struct literals and method dispatch resolve across module boundaries too.
- `core/parser`'s `semantic_check_with_env` is unchanged; a new `semantic_check_with_env_and_modules` (and `core/parser/src/semantic`'s `run_semantic_check_with_modules`) is additive, used by `pata/cli`'s pipeline.

### Fixed

- Boolean output for `Ukweli` cast to `Neno`: `false` now prints as `"si_kweli"` (was `"sikweli"`).
- Unreachable code in `mfumo::toka()` on non-WASM targets.
- Test fixtures and examples updated for modular stdlib (e.g. `leta matumizi` where `chapisha` is used).
- `mfumo::sikiliza_ishara`/`rejesha_ishara`'s `#[cfg(not(unix))]` fallback never compiled on any non-Unix target (wrong `Value::Tokeo` arity, missing `MapKey` import) — found while cross-compiling to wasm32 for the first time.
- **Module-level `thabiti` constants are now bound at runtime.** Constants were parsed, type-checked, and exported, but never seeded into the evaluator's environment — referencing one by name (even a file's own local constant) failed with `UndefinedVar`. Fixed in `core/evaluator` (`run_function_with_telemetry`/`run_function_with_builtins`); `merge_for_eval` (`pata/cli`) now also merges imported constants' declarations into the merged module, not just their types.
- `pata.lock` previously could not round-trip a path dependency written as `{ path = "..." }` (the flat-line parser choked on the embedded `=`); fixed by the `pata-package` lockfile rewire above.
- `eval_block_impl`/`eval_expr_impl`'s `stacker::maybe_grow` red zone (32KB) was too tight for debug-build stack frames after this cycle's cumulative additions — a 1000-deep nested-expression recursion test overflowed the real stack before `MAX_EVAL_DEPTH`'s guard could trip, in debug builds only (`--release` was unaffected). Widened to 256KB/2MB growth chunks.
- **`kweli`/`si_kweli kama Namba` always evaluated to `0`** regardless of the boolean's value — the shared `as_f64` numeric-coercion helper used by the `Namba` cast had no `Ukweli` arm. Fixed at the cast site (not in the shared helper, to avoid changing arithmetic/comparison behavior elsewhere).
- **`Tokeo.ni_sawa()`** (the `Ok`-check counterpart to `.ni_kosa()`) was unimplemented for the real runtime `Value::Tokeo` representation — only wired for a legacy enum-shaped "Tokeo" nothing in the current evaluator produces. Calling it threw a method-not-found error on any real `Tokeo`.
- **`Type1 kama Type2 kama Type3` (chained casts) written inline as a call argument silently produced no output and no error** — `parse_type()` didn't stop consuming tokens at the `kama` keyword, so the first cast's type parsing swallowed the second `kama` and its type into one bogus, unrecognized type name; the evaluator's unrecognized-type fallback then passed the original value through unchanged into a mismatched-type argument slot with nothing catching it. The same construct split across two statements always worked correctly.
- **Division by zero panicked** instead of producing IEEE-754 `Ukomo`/`-Ukomo`/`Siyo_Namba` as documented — `/` no longer special-cases a zero divisor; the `kama Neno` cast now also renders these as their Swahili names instead of Rust's raw `inf`/`-inf`/`NaN`.
- **Global constants beyond `Ukomo`/`Siyo_Namba`** (`PI`, `E`, `PHI`, `TOLEO`, `JINA_OS`, `SEKUNDE_KWA_SIKU`, and 13 others) were seeded into the runtime environment but rejected by the semantic analyzer — any program referencing one failed to compile despite the doc claiming they're always in scope.
- **Struct and pair (`Jozi`) destructuring patterns didn't bind their variables at the semantic-analysis layer**, even though the evaluator's pattern matcher already bound them correctly at runtime — `linganisha p { Pika { x: a, y: b } => { ...a...b... } }` failed to compile for `a`/`b`.
- **List index-assignment (`a[i] = val`) was unimplemented for `Orodha`** — it always lowered to a call shared with `Kamusi`'s `m[k] = v`, but only the `Kamusi` arm existed; using it on a list threw a runtime type error.
- **`Tokeo::Ok(v)`/`Tokeo::Err(e)`/`Chaguo::Some(v)` patterns in `linganisha` silently never matched a builtin-produced `Tokeo`/`Chaguo` value** — no error, the arm just never ran. Root cause: two separate runtime shapes exist for these (`Value::Tokeo`/`Value::Chaguo` from builtins vs. `Value::Enum` from explicit constructor syntax); pattern matching only ever compared against the `Value::Enum` shape.
- **`Kamusi.idadi()` (entry count) type-checked but crashed at runtime**; `Kamusi.funguo()`'s chained-call return type and `weka_key`'s type contract were missing from the analyzer's method-type table despite both being wired at the evaluator-dispatch level.
- **`umbiza()`'s calendar-date formatting used fixed 30-day months with no leap-year handling** — dates could be off by more than two weeks depending on time of year. Replaced with correct proleptic-Gregorian arithmetic.
- **`Biti16`/`uBiti16` casts never range-checked** — any value "fit," unlike every other fixed-width integer type.
- **`hisabati`'s domain-checked functions (`mzizi`, `faktoriali`, `baki`, `logi*`, `asini`, `akosini`, `akosini_h`, `atanjenti_h`) returned an inconsistent struct-shaped error**, unlike the rest of the module's plain `Neno` error strings.
- `SEM023`'s match-exhaustiveness message used a garbled double-negative/infinitive construction ("linganisha inaweza kutokuwa na kufanya kazi..."); reworded to a grammatically sound sentence. `SEM096` had backwards genitive word order ("uga 'X' aina hailingani"); corrected to "aina ya uga 'X' hailingani...". Several other diagnostic messages across `core/parser` had trailing-space bugs, inconsistent hyphen-vs-em-dash punctuation, or vague wording (e.g. `PAR072`'s invalid-number message now names the offending lexeme) tightened up during a full grammar/descriptiveness pass over user-facing error text.

### Changed (breaking)

- **`Tokeo`/`Chaguo`'s built-in variant names are now Swahili, matching the already-resolved spec** (`docs/spec/08-resolved-decisions.md`, `docs/spec/04-type-system.md`): `Tokeo::Ok`/`Tokeo::Err` → `Tokeo::Sawa`/`Tokeo::Kosa`; `Chaguo::Some` → `Chaguo::Kuna` (`Chaguo::Hamna` unchanged). Any code pattern-matching on the old English names needs updating. Raw/REPL debug-print output changed to match (`Tokeo(Sawa(...))`/`Tokeo(Kosa(...))`/`Chaguo(Kuna(...))`/`Chaguo(Hamna)` instead of the Rust-derived `Ok`/`Err`/`Some`/`None`).
- **`#[sharti(target = "...")]`'s recognized key is now `lengo`**, matching the word already used everywhere else for "build target" (`pata.toml`'s `[jenga] lengo`, the `--lengo` CLI flag). `#[sharti(target = ...)]` now fails with `SHA001: sharti key isiyojulikana: "target"`; use `#[sharti(lengo = "...")]`.
- **The build-output directory is now `kilele/`, not `target/`** (applies to Asili *project* build output only — this repository's own Rust/Cargo `target/` is unaffected). `pata njozi` now scaffolds `kilele/` + a matching `.gitignore`; `pata jenga`/`pata tenda` write/read `kilele/`. Existing projects with a `target/` directory should rename it (or just let `pata jenga` recreate `kilele/`, then delete the old `target/`).
- **CLI flags are now Swahili-only, with no English fallback**: `--check` → `--kagua` (`nadhifu`), `--filter` → `--chuja` (`jaribu`), `--fail-fast` → `--simama-haraka` (`jaribu`); the English aliases for `--help`/`-h`, `--target`, `--out`, `--profile` were removed (use `--msaada`, `--lengo`, `--pato`, `--namna` — these already worked as the Swahili spelling, they just also silently accepted English before).
- **`.build.manifest` file format keys renamed to Swahili**: `project`/`entry`/`functions`/`artifact`/`input_hash` → `mradi`/`kuingia`/`kazi`/`kilele`/`hashi_chanzo`. Anything parsing this file directly (not via `pata tenda`) needs updating.
- **Diagnostic `stage` tags (shown in every compile-error line, e.g. `kosa: semantic: ...`) are now Swahili**: `parse`/`semantic`/`evaluator`/`lint`/`resolve`/`lex`/`general` → `uchanganuzi`/`semantiki`/`kitekelezi`/`ukaguzi`/`utatuzi`/`leksika`/`jumla`. Anything grepping/matching on the English stage string in error output needs updating.

## [0.1.0] (initial implementation)

- Core: lexer, parser, semantic analysis, evaluator, diagnostics.
- CLI (Pata): `jenga`, `jaribu`, `njozi`, `nadhifu`, `thibitisha`, `mwalimu` (LSP).
- Stdlib (built-in): msingi, mfumo, majira, matumizi, faili, hisabati.
- Types: Namba, Neno, Ukweli, Tupu, Hamna, Chaguo, Tokeo, Orodha, Kamusi, Jozi, Struct, Wakati, Herufi, bit-width casts.
- Control flow: ikiwa/au_ikiwa/vinginevyo, wakati, kwa, linganisha, vunja/endelea, jaribu/?.
- Examples and spec-aligned layout.

[Unreleased]: https://github.com/.../compare/v0.2.0...HEAD
[0.2.0]: https://github.com/.../compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/.../releases/tag/v0.1.0
