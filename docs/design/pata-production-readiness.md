# Pata toolchain: production-readiness roadmap

Scope: `pata/` only (`cli`, `lsp`, `fmt` [logic lives in `cli/src/pipeline/format.rs`], `lint`,
`package`, `runner`, `dap`). `core/` (lexer/parser/semantic analyzer/evaluator) is treated as a
given — see [implementation-status.md](implementation-status.md) for that layer's own gaps. The
one deliberate exception: `core/evaluator`'s `DebugHook` trait/`RealDebugHook` implementation
(item 7, DAP) — real step-through debugging needed a hook inside the interpreter's own
statement-execution loop, which is `core/`-side work by necessity, tracked here rather than
treated as purely out of scope since `pata-dap`'s own completion depended on it.

This doc sets the **floor** (what must be true before calling the toolchain production-ready) and
a **stretch tier** (what would make it good, benchmarked against how comparable single-binary
toolchains — Cargo, Gleam, Zig — solved the same problems). Every floor item is tied to a specific,
verified gap in this codebase, not a generic checklist.

---

## Why these two tiers, and why this order

The floor is deliberately narrow: it's the set of gaps where the toolchain currently either lies
about what it did (silent data loss, e.g. formatter corruption) or blocks the single most common
toolchain operation (add a dependency, actually get its code). Everything else — nicer resolver
diagrics, incremental LSP, richer lint — is real but doesn't block calling this "production ready"
for the workflows the toolchain currently advertises.

External benchmarking (see below) mattered most for one decision: **whether "no registry" is
disqualifying.** It isn't — Zig shipped 1.0-track tooling for years on content-hash-pinned
git/tarball URLs with no central registry at all, and only in 2026 is the ecosystem debating
whether it ever needs one. That reframes item 1 below: the floor is not "stand up a registry
server," it's "make `pata ongeza` actually fetch something and verify it," which is a much smaller
and more achievable bar.

---

## The floor (blocking "production ready")

### 1. `pata ongeza` must actually fetch and verify a dependency's code — DONE

**Update:** fully wired now, confirmed via direct source read (`pata/cli/src/commands/
ongeza.rs`), not just the earlier-landed library primitives. `pata ongeza <lib> --git <url>
[--tawi <branch>]` calls `pata_package::fetch_git` for real — a real `git2`-based clone into
`.asili/packages/<lib>/`, a real SHA-256 content hash over the fetched tree written into
`pata.lock`. A bare `pata ongeza <lib>` (no `--git`) resolves against the real local/hosted
registry (item 6, stretch tier) instead of a name-string placeholder. `pata jenga` re-verifies
every vendored dependency's content hash against `pata.lock` before every build
(`LockFile::verify_content_integrity`, called from `pata/cli/src/pipeline/project.rs`) — a
tampered or swapped `.asili/packages/<name>/` directory is a hard build error, the actual
security property a lockfile is for. Verified end-to-end against a real local git repo
(`file://` clone) in `ongeza.rs`'s own test module, not just unit tests against the library
functions in isolation.

**Original gap (historical — kept for context on why this was the floor's #1 item):**
`pata ongeza` used to write an entry to `pata.toml`/`pata.lock` without fetching anything;
`Resolver::resolve` did no I/O and hashed `sha256("{name}@{version}")` (the name string, not
content) — two different tarballs vendored under the same path would have produced identical
checksums, so the lockfile had no real tamper-detection property at all. Path dependencies were
the only mechanism that ever actually worked.

### 2. CI pipeline

**Current state:** confirmed directly — `.github/` contains only `PULL_REQUEST_TEMPLATE.md` and
issue templates, no `workflows/`. Every one of the ~180 passing tests, every lint pass, every
`pata jenga` on the example set only runs when a human remembers to run it by hand.

**Floor fix:** a single workflow, run on every push/PR to `main`/`develop`:
- `cargo build --workspace`
- `cargo test --workspace`
- `pata-lint` across `examples/`
- `pata jenga` (build, not run) on every project under `examples/`

This is the precondition for trusting every other item on this list stays fixed. Cheapest, highest-
leverage item on this doc — a few hours of work, and it's the one every external "production ready"
source (see Sources below) lists as non-negotiable table stakes, not a nice-to-have.

### 3. Formatter must not corrupt source

**Current state:** `canonical_format` (`pata/cli/src/pipeline/format.rs:39-70`) is a line-based text
transform. Its own comments admit: strips all indentation, does blind string-replace on `{`/`}`/`,`
**including inside string literals**, so `chapisha("a, b")` becomes `chapisha("a, b ")` or worse if
the literal contains braces. `pata thibitisha` calls this same formatter as a gate
(`thibitisha.rs`) — meaning the toolchain's own "is this ready to publish" check can silently mangle
a user's source strings and then declare the result correctly formatted.

**Floor fix:** formatter must be token/AST-aware, at minimum enough to never rewrite the contents of
a string or comment token. Full pretty-printing (struct-field alignment, operator spacing) is
stretch-tier; *not corrupting data* is floor. A formatter that corrupts input is worse than no
formatter, because it currently runs silently inside `thibitisha`'s gate.

### 4. `pata thibitisha` must check what it claims to check — DONE

**Original gap:** its own top-of-file comment listed 3 implemented checks (doc coverage, format
compliance, opt-in test-coverage threshold) against a documented 6. Missing: type-stability across
versions, ABI compatibility for `kiungo`/FFI exports, trait-completeness (originally mis-scoped as
"every sifa in `[tegemezi]`" — `[tegemezi]` holds package dependencies, not trait names; the real
gap is traits declared by imported modules with zero impls anywhere).

**Fixed — 2 of the 3 missing checks implemented for real, the 3rd correctly identified as blocked
on infrastructure that doesn't exist yet, not deferred by choice:**
- **Trait completeness**, implemented properly rather than shortcut: `pata/cli`'s `.asi`
  interface-stub parser (`interface_registry.rs`) now extracts `sifa { ... }` blocks (previously
  untracked entirely — only functions/constants were parsed from `.asi` files), converts them to
  real `TraitDecl`s, and threads them through `resolve.rs` into the merged module. `thibitisha`
  then checks every trait reachable from the project (local or via `leta`) has at least one impl
  anywhere — a check `SEM105` (which only fires on an impl that already names a trait and gets it
  wrong) never covered: a trait with **zero** impls compiled and shipped silently before this fix.
- **FFI-safety of `#[kiunganishi]` signatures**, implemented as the correct scoped-down version:
  full ABI-compatibility checking needs a declared C signature to compare against, and none exists
  (`kiungo`/FFI is still a Phase IV stub — confirmed, not assumed). What's real and checkable today
  is the prerequisite: every `#[kiunganishi]`-tagged function's parameter/return types must be
  primitives that could ever be ABI-safe (`Namba`/`Ukweli`/`Herufi`/`Tupu`/`Anuani`/fixed-width
  `Biti*`), rejecting `Orodha`/`Kamusi`/struct/other heap-owning types outright.
- **Type-stability across versions** remains unimplemented — correctly identified as blocked on a
  real prerequisite (a baseline snapshot of a prior published version), not deferred by choice.
  Decided approach: a git-tag (`v<toleo>`) baseline, once item 1's dependency-fetch path exists to
  make versioned publishing meaningful. Tracked as its own follow-up, not silently dropped.

See `pata/cli/src/commands/thibitisha.rs` (`enforce_trait_completeness`/`enforce_ffi_signatures`),
`pata/cli/src/pipeline/interface_registry.rs` (`TraitStub`/`parse_asi_content`'s `sifa` handling),
and `docs/howto/04-validate-docs.md` for the user-facing writeup.

### 5. Semantic-analyzer test coverage under the surface pata drives — DONE (the pata-side fix)

**Update:** `pata_cli::pipeline::compile::tests::every_example_project_builds_with_zero_diagnostics`
now builds every real project under `examples/` (discovering `examples/cross_package`'s nested
`app/` root as the one special case) and asserts `compile_project` succeeds for each — a
regression in the resolver/semantic-checker/formatter layer `pata` depends on is now caught by
`pata`'s own test suite (and therefore CI, item 2) rather than discovered by a user running `pata
jenga` by hand. Lives as a unit test inside `pipeline::compile.rs` itself, not a separate
`pata/cli/tests/*.rs` integration test — `pata-cli` has no `[lib]` target, so an external
integration test can't call `compile_project` at all.

**Original gap:** Technically a `core/` item, but it was on the floor here because it's the one
gap that had already caused **real, user-facing toolchain bugs** — three of the 22 documented bugs
in [implementation-status.md](implementation-status.md) were exactly the class of thing `pata
jenga`/`pata thibitisha` silently shipped wrong (a struct field's type annotation discarded; a
struct/pair destructuring pattern rejected at compile time; module constants unresolvable).
`pata`'s own test suite couldn't catch these because `semantic_check_with_env_and_modules` — the
actual entry point `pata-cli` calls — had zero direct test callers. This fix doesn't mean fixing
all ~50 untested `SEM0xx` codes (that's still `core/`'s job and out of this doc's scope) — only
the `pata`-side integration-test gap, which is what's actually done now.

---

## Stretch tier (raise the bar beyond the floor)

Benchmarked against Cargo, Gleam, and Zig — ordered roughly by leverage, not urgency.

### 6. A real package registry (Gleam/Hex-style) — DONE, local and hosted both

**Update:** `pata_package::LocalRegistry` is a real, working file-based index (one JSON file per
package at `.asili/registry/<name>.json`, each a published version plus a fetchable git/path
source) with real semver constraint solving (`Resolver::resolve` now does direct `semver::
VersionReq` matching, no longer the "lock whatever string is given verbatim" stub) and real
transitive dependency resolution — a registry package's own declared deps are fetched/locked
too, breadth-first, with real version-conflict detection across dependents requiring
incompatible ranges of the same transitive package. A `remote_registry` module adds the hosted
half: `fetch_index`/`fetch_and_verify` fetch a static-file HTTP index (Cargo alternative-registry
RFC minimum surface — a JSON index + tarball download endpoint, no API server) and verify a
downloaded tarball's SHA-256 before extracting, wired into `Resolver::resolve` via a new
`RegistrySource::Http` variant. Confirmed via a real end-to-end resolver test exercising the
`Http` source end to end, not just the module existing in isolation.

### 7. Wire up multi-package workspace support — DONE, unified onto one manifest

`pata jenga` calls `find_workspace_root` (`pata/cli/src/pipeline/project.rs`), which discovers a
workspace root by finding a `pata.toml` whose `[eneo-kazi].wanachama` table is non-empty —
confirmed via direct `grep`. This item's original implementation used a separate `Asili.toml`
manifest (`pata_package::Workspace`); that was later unified into `pata.toml` itself (real TOML
parsing replacing the earlier hand-rolled line scanner, plus a Swahili `[eneo-kazi]` table) so a
workspace root and an ordinary project share one manifest file and one syntax — see
[package-manager-design.md](package-manager-design.md) for the current design.
`pata_package::Workspace`/`WorkspaceConfig`/`Manifest` were deleted once nothing outside their own
tests referenced them. `pata ongeza` itself is not workspace-aware yet (still resolves against the
single project's `pata.toml`); that's the remaining gap if workspace-scoped dependency addition is
needed later.

### 8. LSP incremental re-resolution — PARTIALLY DONE (closes most of #25)

**Update:** Three real, scoped fixes have landed (`pata-implementation-spec.md` Section 19's
decision — real, targeted caching at three layers, not a full salsa rewrite):
`did_change_watched_files` invalidates only the specific project root(s) whose files actually
changed (`workspace::affected_project_roots`), not the entire cross-file resolution cache;
`DocStore::diagnostics_for` skips re-lex/re-parse/re-analyze on a `didChange` whose text hashes
identically to what's cached; and (new) `workspace::ModuleCache` caches each project-local file's
parsed `WorkspaceModule` keyed by its own content hash, so a re-walk of a project's import graph
(triggered by `did_change_watched_files` evicting that root) reuses every unchanged file's
already-parsed module instead of re-tokenizing/re-parsing it — verified directly via a real
parse-count counter in two tests (`resolve_workspace_reuses_cached_module_without_reparsing_
unchanged_file`, `resolve_workspace_reparses_a_file_whose_content_actually_changed`). What's still
missing, genuinely: `ModuleCache` closes the "re-parse every file" gap but a re-walk still
traverses the *whole* import graph from the entrypoint on every cache-refreshing call — there's no
query that starts from "what depends on the one file that changed" and works outward, true
per-file salsa-style recomputation. That remainder is the actual rust-analyzer-reference-
architecture-sized item, not the caching wins above.

**Original gap:** Mwalimu (`pata/lsp`) is the toolchain's most complete piece — diagnostics,
hover, completion, goto-def, references, rename, workspace symbols. Its own doc named the gap:
every edit triggers a full workspace re-check, no incremental model. Not urgent (correctness >
speed here), but the ceiling on "feels production-grade" for any project past a few dozen files.

### 9. Lint rule depth

7 rules across `naming.rs` (70 lines), `style.rs` (26 lines), `best_practices.rs` (120 lines).
LINT201's "more than 5 string literals = repeated" bug was already found and fixed this cycle
(per implementation-status.md) — the other rules haven't had equivalent scrutiny. Stretch-tier:
audit each rule against a deliberately-crafted false-positive/false-negative test file, the way
LINT201 should have been tested from the start.

### 10. AST-based formatter (full pretty-printing)

Once item 3's floor (don't corrupt data) is met, the stretch version is a real pretty-printer:
operator spacing, struct-field/match-arm alignment, configurable via a `pata.toml` section — the
way `rustfmt.toml`/Gleam's zero-config formatter work. Gleam's choice (opinionated, no config) is
worth defaulting to over rustfmt's many knobs, given Asili's own small-config-surface philosophy
elsewhere (e.g. `pata.toml`'s minimal sections).

---

## Recommended order of work

1. **CI pipeline** (item 2) — cheapest, unblocks trusting every other fix stays fixed. Do this
   first, before touching anything else on this list. **Done** — `.github/workflows/ci.yml`.
2. **`pata thibitisha` trait-completeness + FFI-safety checks** (item 4) — **done**, ahead of its
   original position in this list, since it turned out smaller than items 1/5 once scoped
   correctly (see item 4 above for what shipped vs. what's genuinely still blocked).
3. **`pata ongeza` real fetch + content-hash verification over git/path sources** (item 1) — the
   single highest-value remaining functional gap; makes dependency management real. Also unblocks
   item 4's remaining type-stability check (needs a git-tag baseline).
4. **Formatter data-safety fix** (item 3) — small, contained, prevents active data loss.
5. **`examples/`-driven integration tests in `pata-cli`'s own suite** (item 5) — needs item 2 (CI)
   to have teeth.
6. Stretch tier in the order listed above, registry (6) first since it's the natural continuation
   of item 3's groundwork.

---

## Full build-out (beyond floor/stretch: what "complete," not MVP, looks like per tool)

The floor/stretch tiers above set a minimum bar. This section is different: for every `pata/`
tool, what would it take to fully satisfy its own documented contract — not "good enough to call
production ready," but genuinely done. Sequenced strictly by dependency order (not by tool name
or perceived importance), with relative effort sizing. Two structural findings changed the
sequencing versus a naive per-tool list:

**`pata-lsp` cannot depend on `pata-cli`.** `pata-cli` is a binary-only crate (no `lib.rs`), and
it already depends on `pata-lsp` to launch `pata mwalimu` — so the reverse edge would cycle. The
LSP's own doc comments confirm this was a deliberate, known tradeoff: `pata/lsp/src/workspace.rs`
(lines 21-26) is a **second, smaller, independent reimplementation** of `pata-cli`'s module
resolver — no version-dependency resolution, and critically, it won't see the `.asi`
trait-tracking added to `interface_registry.rs` (see the "Fixed" write-up under item 4 above)
unless someone duplicates it there too. `pata/lsp/src/format.rs` (line 7) is a literal copy-paste
of `canonical_format`, its own comment reading "inlined ... to avoid circular dependencies." This
means **any "full" work on `pata-cli`'s resolver, formatter, or interface registry silently does
not reach the LSP** unless a shared library crate is extracted first — hence item 0 below, ahead
of the formatter and package-manager work that would otherwise need to be built twice.

**The bytecode VM is a `core/evaluator` subsystem, not a `pata/` one.** `pata tenda` (`pata/
runner`) already dispatches to `run_bytecode` when given a bytecode-format `.asb` — the consuming
side is fine. The gap is entirely upstream: `core/evaluator/src/bytecode.rs`'s ISA is an admitted
skeleton (its own comment: `TODO(Phase II/IV): missing opcodes needed for real programs`), and
nothing in `pata jenga`'s pipeline ever emits that format. Per this doc's established scope
(`pata/` toolchain only, `core/` treated as a given), the VM itself is out of scope here; only the
small `pata jenga` wiring change once the VM is real would be in scope, called out as externally
blocked rather than silently included in sizing below.

### Dependency graph

```
0. Shared core crate (pata-core)
   └─ unblocks: 1 (formatter reuse), 7 (LSP resolver reuse), 7 (LSP formatter reuse)

1. AST-based formatter (pata-fmt rewrite)
   └─ unblocks: 5 (thibitisha's format gate becomes trustworthy),
                 7 (LSP format-on-save inherits it via pata-core)

2. pata ongeza real fetch + content-hash verification (git/path sources)
   └─ unblocks: 3 (real resolver needs something to resolve against),
                 4 (workspace wiring needs real deps to matter),
                 5 (thibitisha's type-stability check needs tagged, fetchable versions)

3. Real dependency resolver (semver constraint solving) + registry backend
   └─ builds on 2

4. Workspace (pata.toml [eneo-kazi], unified single-manifest design) wired into pata jenga/ongeza
   └─ independent of 2/3 internally, but only valuable once 2 makes deps real

5. pata thibitisha type-stability check (git-tag baseline)
   └─ builds on 2 (needs real, fetchable tagged versions to diff against)

6. pata jaribu full build-out (parallel exec, timeouts, fixtures, structured output, coverage)
   └─ independent — no hard dependency on 0-5, sequenced here because it's medium-sized
      and self-contained; could move earlier without breaking anything

7. Mwalimu (LSP) full build-out (incremental resolution, inlay hints, code actions,
   workspace-wide rename, DAP)
   └─ partially builds on 0 (resolver/formatter reuse); DAP is a wholly separate protocol
      server, sequenced last within this item since nothing else depends on it

8. pata-lint rule depth + new rules + configurability
   └─ independent — sequenced last only because it's the lowest-leverage, most
      "add more of the same" item; no downstream blockers

9. pata njozi template/scaffold improvements
   └─ independent, small; benefits from 4 (workspace scaffolding) and the CI work
      already done (can generate a workflow stub), sequenced after those land
```

`core/evaluator` bytecode-VM completion is called out separately at the end as **externally
blocked, not sequenced** — revisiting the `pata/`-only scope boundary to include it is a decision
for whoever picks this doc up next, not assumed here.

### Sizing legend

- **S** — days, single-crate, no design ambiguity
- **M** — 1-2 weeks, touches 2-3 crates or needs a real design decision
- **L** — multi-week, genuinely new subsystem, needs its own design doc before implementation

### 0. Extract `pata-core` shared library crate — **M**

New crate (`pata/core` or similar) holding: the module resolver (`resolve.rs`,
`interface_registry.rs` from `pata-cli`, including the `.asi` trait-tracking added this cycle),
and eventually the formatter (once rewritten, item 1). `pata-cli` becomes a thin binary over
`pata-core`; `pata-lsp` depends on `pata-core` directly, retiring `workspace.rs`'s parallel
reimplementation and `format.rs`'s copy-paste. Risk: this is a structural refactor of code that
has its own existing tests (`pata-cli`'s `pipeline::compile`/`resolve` test modules) — full
completeness means those tests move/adapt cleanly, not a rewrite from scratch.

### 1. AST-based formatter — **L**

Real pretty-printer visitor over the parsed `Module` AST (not a text transform): operator
spacing, re-indentation, struct-field/match-arm alignment, comment-preserving. Needs an explicit
config-surface decision first (zero-config Gleam-style vs. a `pata.toml` `[fmt]` section) — an
open decision, not resolved in this doc. Needs a golden-file test suite across real syntax
(structs, traits, generics, match arms, every attribute form) replacing the current single
`format_is_idempotent` test, plus a `--diff` mode alongside the existing check-only mode. Once
item 0 exists, this is implemented once and both `pata nadhifu` and the LSP's format-on-save
consume it — the reuse payoff for sequencing this after 0.

### 2. `pata ongeza` real fetch + content-hash verification — **L**

Git-clone and/or tarball fetch for `Dependency::Version`/`git` manifest entries into
`.asili/packages/<name>/`. Real SHA-256 over fetched content (not `sha256("{name}@{version}")`,
the current `compute_checksum` in `pata/package/src/resolver.rs`), written to `pata.lock`,
verified before every build (`pata jenga`, or a new `pata sasisha` step). No registry server
required for this item (per the git-tag/git-source precedent in floor item 1 above) — git/path
sources are the target here. `pata ondoa <lib>` (remove) added alongside, since it's the natural
counterpart and doesn't exist at all today.

### 3. Real dependency resolver + registry backend — **L**

Replace `Resolver::resolve`'s verbatim-lock-whatever-is-given logic with real semver-range
constraint solving (`^1.2`, `>=1.0, <2.0`) and transitive-conflict detection; wire up the
currently-unused `_existing_lock` parameter for actual incremental re-resolution. Registry
backend: minimum viable surface per Cargo's alternative-registry RFC (JSON index + tarball
download, no API server required) — self-hostable (Kellnr/Alexandrie-style), not necessarily a
hosted service Asili itself runs. Explicitly the largest single item in this plan; needs its own
design doc before implementation starts.

### 4. Workspace support wired into `pata jenga`/`pata ongeza` — **DONE (unified onto `pata.toml`)**

Resolved: `pata.toml` gained its own `[eneo-kazi]` (Swahili "workspace") table
(`wanachama = [...]`) instead of requiring a separate `Asili.toml` file. This meant migrating
`load_project_config`'s parser from a hand-rolled line scanner to a real TOML library
(`toml::Table`) — needed anyway to represent `[eneo-kazi]`'s nested array correctly, and a
byproduct benefit is malformed `pata.toml` now errors instead of silently skipping unparseable
lines. `find_workspace_root` discovers a workspace root by walking upward for a `pata.toml` with a
non-empty `[eneo-kazi]`; `pata jenga --workspace-info` and `pata njozi --workspace` both use the
unified format. `pata_package::Workspace`/`WorkspaceConfig`/`Manifest` (the old `Asili.toml`-only
types) were deleted once nothing outside their own tests referenced them. See
[package-manager-design.md](package-manager-design.md) for the full design. `pata ongeza` itself
remaining not workspace-aware is unchanged by this — still a real gap, tracked in item 7 above.

### 5. `pata thibitisha` type-stability check — **M**

Git-tag (`v<toleo>`) baseline lookup: `git show <tag>:src/**/*.as` → parse → collect public
signatures → diff against current. Flags: removed public item, changed param/return type, changed
arity, changed trait signature (extends beyond functions, covering the trait/struct-change gap
noted under item 4's "not yet checked" above).

### 6. `pata jaribu` full build-out — **M**

Parallel test execution (worker-pool model, a `--test-threads`-equivalent flag); per-test
timeout; setup/teardown fixture hooks; structured output (JSON/JUnit XML) for CI dashboard
integration; code coverage instrumentation (line/branch — distinct from `thibitisha`'s existing
function-count-ratio check); richer assertion helpers beyond bare `paparika`.

### 7. Mwalimu (LSP) full build-out — **L**

True incremental re-resolution (salsa-style), replacing full-workspace-recheck-on-every-edit.
Inlay hints (inferred types, parameter names at call sites) — explicitly named as missing in the
crate's own doc comment. Code actions/quick-fixes beyond the one existing doc-stub insertion.
Workspace-wide rename needs explicit verification (currently only confirmed single-document).

**DAP (debugger): done.** `pata-dap`'s protocol layer (already complete) is now backed by a real
`DebugHook` implementation in `core/evaluator` (`debug_hook::RealDebugHook`, wired into
`eval_stmt_impl` via `Runtime::debug_hook`) instead of only `MockHook` — a `launch`+
`setBreakpoints`+`configurationDone` DAP sequence genuinely compiles and runs a target `.as` file,
pauses it at a real breakpoint, and reports real live variable bindings. Verified via a real
integration test (`pata/dap/src/runner.rs`) and manually against the compiled binary over a live
stdio pipe. See [dap-later.md](dap-later.md) for the full write-up. Single-file `launch` only (no
project/dependency-aware compilation) remains a real, documented gap.

### 8. `pata-lint` rule depth + new rules — **M**

Adversarial false-positive/false-negative fixture test per existing rule (LINT001-003, LINT101,
LINT202, LINT203) — same rigor LINT201 got this cycle, extended to the rest. New rule categories:
unused variables/dead code beyond imports, shadowing warnings, unreachable code after `rejesha`,
redundant `linganisha` arms. Per-rule configurability (severity, thresholds like LINT101's
hardcoded 50-line cutoff) via `pata.toml` or inline suppression syntax. Machine-readable (JSON)
output for editor/CI integration.

### 9. `pata njozi` template/scaffold improvements — **S**

Template variants (library vs. binary). Workspace scaffolding (multi-package `[workspace]`
layout), meaningful only after item 4. Generate a `.github/workflows/ci.yml` stub alongside new
projects, dogfooding the CI work already merged to this repo.

### Called out, not sequenced: `core/evaluator` bytecode VM completion

Out of this doc's `pata/`-toolchain scope. Noted here as the reason `pata jenga` can't emit real
bytecode today, with a pointer to `core/evaluator/src/bytecode.rs`'s own `TODO(Phase II/IV)`
comment, so the dependency is documented rather than silently absent from this plan.

---

## Sources consulted

- [Testing and CI / production-readiness checklist practices](https://gentleduck.org/duck-registry-build/testing-ci) — CI/lockfile/test-setup as baseline gates.
- [Cargo Book: Dependency Resolution](https://doc.rust-lang.org/cargo/reference/resolver.html) and [Registry Index](https://doc.rust-lang.org/cargo/reference/registry-index.html) — constraint-solving resolver and minimal registry-index API shape.
- [Rust RFC 2141: Alternative Registries](https://rust-lang.github.io/rfcs/2141-alternative-registries.html) — minimum viable self-hosted registry surface (JSON index + tarball fetch, no API server required).
- [Gleam usage — Hex.pm](https://hex.pm/docs/gleam-usage) and [gleam.toml docs](https://gleam.run/writing-gleam/gleam-toml/) — manifest/lockfile split and single-binary integrated toolchain as the closest peer design to Pata's ambition.
- [Zig and the M×N Supply Chain Problem (Andrew Nesbitt, 2026)](https://nesbitt.io/2026/01/29/zig-and-the-mxn-supply-chain-problem.html) — content-hash-pinned manifest-as-lockfile with no central registry, the precedent for scoping item 1's floor down from "build a registry."
- [Kellnr — self-hosted Rust crate registry](https://blog.octabyte.io/posts/development/kellnr/kellnr-host-private-rust-crates-on-your-own-hardware-with-full-control/) — reference for a lightweight self-hostable registry implementation if item 6 is pursued.
