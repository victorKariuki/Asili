# Package manager (Pakiti/Moduli) design

The Asili package manager is split across two crates: `pata/package` (the `pata-package`
library — manifest/lock/resolve/workspace primitives) and `pata/cli`'s
[`pipeline/project.rs`](../../pata/cli/src/pipeline/project.rs) and
[`pipeline/resolve.rs`](../../pata/cli/src/pipeline/resolve.rs) (the CLI-facing layer that
actually drives `pata jenga`/`pata ongeza`/module resolution). This doc covers the rewire that
replaced a hand-rolled resolver/lockfile with `pata-package`, superseding the two-bullet summary
in [implementation-status.md](implementation-status.md#phase-ii--synthesis-lsp-wasm-tooling).

## The core architectural decision: the adapter approach

`pata-package` defines its own manifest dialect — English keys, serde-derived, TOML file named
`Asili.toml` (`pata/package/src/manifest.rs:1,9-37`: `WorkspaceConfig`/`Manifest` with
`[package]`/`[dependencies]`/`[workspace]` sections, `Manifest::load`/`from_str`/`save` all
reading/writing `Asili.toml`).

The on-disk project manifest that `pata` projects actually use is a **different, older format**:
`pata.toml`, with Swahili section/key names (`[jumla]`/`jina`/`toleo`/`asili`, `[chanzo]`/`kuingia`,
`[tegemezi]`, `[jenga]`/`lengo`), parsed by a hand-rolled line scanner in
`load_project_config` (`pata/cli/src/pipeline/project.rs:45-130`) — not TOML-crate parsing, not
serde, no dependency on `pata_package::Manifest` at all.

**These two formats were not unified.** `pata.toml` was kept exactly as-is; `pata_package`'s
`Asili.toml`/`Manifest`/`WorkspaceConfig` types are not read from or written to disk by any CLI
command (`Workspace::open`/`Manifest::load` have no callers under `pata/cli/src/` — confirmed by
grep, the only consumers are `pata-package`'s own unit tests). Instead, the CLI **adapts** its own
`Dependency` enum (`pata/cli/src/pipeline/project.rs:6-16`, `Version(String)` / `Path(PathBuf)`,
parsed straight out of `pata.toml`'s `[tegemezi]` section) into `pata_package::manifest::Dependency`
at the point resolution/locking is needed:

```rust
// pata/cli/src/pipeline/project.rs:219-229
fn to_package_dependency(dep: &Dependency) -> pata_package::manifest::Dependency {
    match dep {
        Dependency::Version(v) => pata_package::manifest::Dependency::Version(v.clone()),
        Dependency::Path(p) => pata_package::manifest::Dependency::Table(pata_package::manifest::DependencyTable {
            version: "0.0.0".to_string(),
            path: Some(p.to_string_lossy().to_string()),
            git: None,
            branch: None,
        }),
    }
}
```

`write_lockfile` (`project.rs:254-274`) builds a `BTreeMap<String, pata_package::manifest::Dependency>`
via this adapter, hands it to `pata_package::Resolver::resolve`, and gets back a
`pata_package::LockFile` that it saves as `pata.lock`. `read_lockfile` (`project.rs:235-251`) does
the reverse: loads `pata.lock` through `pata_package::LockFile::load`, then converts each
`LockedDependency` back into the CLI's own `Dependency` enum.

**Why an adapter instead of migrating `pata.toml` to `Asili.toml`:** a manifest-format migration is
a breaking change to every existing project file, the Swahili keyword surface (`[jumla]`, `[tegemezi]`,
`[jenga]`/`lengo`) is part of the language's documented identity (see
[docs/spec/02-architecture-and-files.md](../spec/02-architecture-and-files.md)), and `pata.toml`'s
custom line-based parser already has its own semantics (path-dependency table syntax, comments,
section ordering) that would all need re-verifying under serde/toml. Converting `Dependency` values
in memory at the two call sites that need `pata_package`'s types is much lower risk than changing
the on-disk format. The cost is that `pata_package::Manifest`/`WorkspaceConfig`/`Workspace`
(`pata/package/src/workspace.rs`) — the multi-package `[workspace]` support — are currently dead
code from the CLI's perspective: nothing in `pata/cli` reads `Asili.toml` or calls `Workspace::open`,
so workspace-of-packages support doesn't exist yet for real `pata.toml` projects even though the
library-level plumbing for it is written and unit-tested.

## Module layout: `pata/package/src/`

Declared in [`lib.rs`](../../pata/package/src/lib.rs:4-14):

- **`manifest.rs`** (259 lines) — `Asili.toml` parsing/serialization: `WorkspaceConfig`, `Manifest`,
  `PackageMetadata`, and the `Dependency` enum (`#[serde(untagged)]` over `Table(DependencyTable)` /
  `Version(String)`, table tried first). `Dependency::version()`/`path()`/`git()` accessors. This is
  the type the CLI adapter (above) converts into and out of — its `load`/`save`/`from_str` methods
  are exercised only by this crate's own tests, not by any CLI code path.
- **`resolver.rs`** (71 lines) — `Resolver::resolve(dependencies, existing_lock) -> Result<LockFile>`.
  See "What actually works" below — this is the whole resolution algorithm, and it is much simpler
  than the name suggests.
- **`lock.rs`** (235 lines) — `LockFile`/`LockedDependency`/`LocalPackage`, TOML load/save
  (`pata.lock` on disk, `version`, `locked_at` RFC3339 timestamp, `dependencies` map, `local_packages`
  map), plus `workspace_checksum()` (SHA-256 over all locked checksums, for detecting whether the
  locked set changed — not currently called from `pata/cli`).
- **`paths.rs`** (192 lines) — `Paths::new(root)` computes the fixed directory layout: `.asili/`
  (package-manager state), `.asili/packages/` (vendored external packages), `.asili/kilele/` (build
  cache), `kilele/` (build artifacts), `lib/`, `src/`. `package_src_path(name)` — used by
  `pata/cli`'s resolver (below) to find a vendored dependency's source — and `init()`
  (creates the directories), `create_gitignore()`.
- **`workspace.rs`** (173 lines) — `Workspace::open(root)` loads `Asili.toml`, validates
  `[workspace] members` paths exist and each has its own `Asili.toml`, aggregates dependencies
  across members. Fully implemented and unit-tested, but — per the adapter-approach section above —
  not wired into any `pata/cli` command; there is no `pata` subcommand that opens a `Workspace`.
- **`lib.rs`** (14 lines) — re-exports: `Manifest`, `Dependency`, `WorkspaceConfig`, `LockFile`,
  `LockedDependency`, `Resolver`, `Workspace`, `Paths`.

## How `pata/cli` delegates to `pata_package`

Two pipeline files drive this, both under `pata/cli/src/pipeline/`:

**`project.rs`** owns the `pata.toml`/`pata.lock` file I/O and the adapter:
- `load_project_config` — parses `pata.toml` (Swahili dialect, hand-rolled, unrelated to
  `pata_package::Manifest`).
- `write_lockfile` (`project.rs:254-274`) — converts `cfg.dependencies` via `to_package_dependency`,
  calls `pata_package::Resolver::resolve(&pkg_deps, existing.as_ref())`, and — as a CLI-level
  behavior not present in `pata_package` itself — preserves the previous `locked_at` timestamp when
  the resolved dependency set is byte-identical to the existing lock, so re-running `pata jenga`/
  `pata ongeza` without dependency changes doesn't churn `pata.lock`.
- `read_lockfile` (`project.rs:235-251`) — loads `pata.lock` via `pata_package::LockFile::load` and
  converts each `LockedDependency` back to the CLI's `Dependency` enum (`path` present → `Path`,
  otherwise `Version`).

**`resolve.rs`** owns module resolution (turning `leta <name>` imports into loaded/merged ASTs) and
is where locked dependencies actually get used to find source files:
- `find_module_file` (`resolve.rs:61-105`) checks, in order: path dependencies (`dependencies.get(name)`
  is `Dependency::Path`, looks for `<path>/src/<name>.as`); then, for a `Dependency::Version` entry,
  the **vendored package cache** via `pata_package::Paths::new(root).package_src_path(name)` joined
  with `<name>.as` (i.e. `.asili/packages/<name>/src/<name>.as`); then falls through to local
  `<root>/<name>.as`, `<root>/lib/<name>.as`, `<root>/lib/<name>/mod.as`, and stdlib
  `<root>/lib/std/<name>.asi`.
- `compile_project` (`pipeline/compile.rs:89-93`) calls `load_project_config`, then overwrites
  `cfg.dependencies` with `read_lockfile(root)`'s result when `pata.lock` exists — so a build with a
  lockfile present resolves modules against the **locked** versions/paths, not whatever
  `pata.toml`'s `[tegemezi]` currently says (mirroring Cargo's `Cargo.lock` precedence).
- `resolve_all`/`resolve_one` do the actual recursive import walk (cycle detection via `RES001`,
  missing-module via `RES002`) and call `merge_for_eval`, which delegates to
  `asili_parser::merge_modules` (`core/parser/src/module_merge.rs:15`) — shared with `driver/wasm`'s
  in-memory bundler so the merge rules live in exactly one place (see item 5 below).

`pata ongeza` (`pata/cli/src/commands/ongeza.rs:13-25`) ties it together at the command level:
`update_dependency` (rewrites `[tegemezi]` in `pata.toml`) → `load_project_config` → `write_lockfile`.
Its own top-of-file comment is explicit about the current ceiling: **`pata ongeza` writes the
dependency to `pata.toml` and generates `pata.lock` but does not download or resolve the package** —
no registry URL scheme, no fetch, no vendor-directory population exists (`ongeza.rs:9-12`).

## What actually works vs. what's explicitly not implemented

**Works:**
- **Path dependencies** (`{ path = "..." }` in `[tegemezi]`) — resolved directly against the
  filesystem path, no lockfile/vendoring involved (`find_module_file`, `resolve.rs:67-78`).
- **Locally-vendored version dependencies** — a `Dependency::Version` entry resolves if (and only
  if) something has already placed its source under `.asili/packages/<name>/src/<name>.as` by some
  out-of-band means; `find_module_file`'s comment says this explicitly (`resolve.rs:80-83`: "populated
  out-of-band ... there is no registry/fetch backend yet"). If it isn't there, resolution falls
  through the remaining candidates and ultimately reports `RES002` with the full searched-path list.
- **Lockfile generation and reuse** — `pata.lock` is real TOML (`pata_package::LockFile`), written
  deterministically (verified by `write_lockfile`'s own test, `project.rs:284-304`,
  `lockfile_is_deterministic`), and consulted by `compile_project` in preference to `pata.toml`'s
  live `[tegemezi]` when present.

**Not implemented — confirmed directly in the code, not inferred:**
- **No registry backend anywhere in this codebase.** `pata_package::Resolver::resolve`
  (`pata/package/src/resolver.rs:13-30`) does no I/O beyond the lockfile object it builds in memory:
  for every `(name, dep)` in the input map it computes a checksum and inserts a `LockedDependency` —
  no HTTP client, no registry index lookup, `_existing_lock` isn't even read (the parameter is
  prefixed `_`, unused). `Cargo.toml` for `pata-package` (`pata/package/Cargo.toml`) has no
  `reqwest`/`ureq`/any HTTP dependency at all — confirmed by grep across the crate. The `source`
  field on a `LockedDependency` is set to `"git"` if the dependency had a `git` URL or `"registry"`
  otherwise (`resolver.rs:24`), but nothing ever acts on that field to actually fetch from either —
  it's descriptive metadata only.
- **No git fetching.** Same evidence as above — `DependencyTable.git`/`.branch` fields exist on the
  manifest type (`manifest.rs:69-71`) and are threaded through, but nothing clones a repo.
- **Checksum is not a content hash of package data** — see the checksums section below; it can't
  detect tampering with vendored source, only that the resolved `name@version` pair matches what was
  locked.
- **`pata_package::Workspace`/multi-package `[workspace]` support** is unused by any CLI command (see
  the adapter-approach section) — writing an `Asili.toml` with `[workspace]` has no effect on
  anything `pata` does today.

## Two real bugs this rewire fixed

**(a) Public module constants were never exported.** `Constant` (`core/parser/src/ast.rs:16-22`) has
no `is_public` field at all — unlike `Function`/`StructDecl`/`TraitDecl`/`EnumDecl`, which all carry
one. There is no visibility modifier on constants in the language yet
(`resolve.rs:3` doc comment: "there is no visibility modifier on constants yet, so every one is
treated as exported"). The actual old bug was that `build_export_table`
(`resolve.rs:43-58`) and `merge_modules`'s constant-handling arm
(`core/parser/src/module_merge.rs:36-44`) didn't consistently export every module-level `thabiti`
constant — some path silently dropped them. Both are now unconditional: `build_export_table` inserts
every constant into the export table with no `is_public` gate (there's nothing to gate on), and
`merge_modules`'s `include` check for constants is `None => true` (`module_merge.rs:38`, contrast
with the `f.is_public`/`s.is_public`/`t.is_public` checks used for functions/structs/traits on the
surrounding lines) when the import is a full `leta X` (not a selective `leta { a, b } kutoka X`).
This is now verified fixed and covered by `core/evaluator/tests/module_constants.rs`.

**(b) `merge_for_eval`/`merge_for_semantic` only merged imported functions, not structs/traits/impls.**
Confirmed fixed: `asili_parser::merge_modules` (`core/parser/src/module_merge.rs:15-77`), which both
`resolve.rs`'s `merge_for_eval` and `driver/wasm`'s bundler call, merges `functions` (lines 27-35),
`constants` (36-44), **`structs`** (45-52), **`traits`** (54-61), and **`impls`** (63-67) — impls are
merged unconditionally by target-type dedup (no visibility check, since an impl block has no name of
its own to select on). `resolve.rs:1-6`'s file-level comment confirms this was a real gap that's
closed: "merge_for_eval merges structs/traits/impls from imported modules in addition to functions —
both were once TODOs here but are already implemented below."

## Checksums and the lockfile

`pata_package::Resolver::resolve` computes a checksum per dependency via `compute_checksum(name,
version)` (`pata/package/src/resolver.rs:33-40`):

```rust
fn compute_checksum(name: &str, version: &str) -> String {
    use sha2::{Digest, Sha256};
    let input = format!("{}@{}", name, version);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}
```

This hashes the **`name@version` string**, not any actual package content/bytes — it is
deterministic (same name+version always produces the same checksum, verified by
`resolver.rs`'s own `test_checksum_deterministic`) and truncated to the first 16 hex characters. It
guards against a `pata.lock` entry silently drifting from what `pata.toml` currently declares (same
role as Cargo's checksum for that narrow purpose), but it is **not** a content-integrity check on
vendored source — two different tarballs/directories placed under `.asili/packages/<name>/` for the
same `name@version` would produce identical checksums. `LockFile::workspace_checksum()`
(`lock.rs:106-122`) is a second, higher-level SHA-256 over all per-dependency checksums plus all
`local_packages` checksums, intended to detect "did the locked set change at all" — it exists and is
tested but has no caller in `pata/cli` today.

The lockfile itself (`pata/package/src/lock.rs:11-20`) is TOML: `version` (schema version string,
currently `"1"`), `locked_at` (RFC3339 timestamp via `chrono`), `dependencies` (map of name →
`LockedDependency { version, checksum, path: Option<String>, source }`), and `local_packages` (map of
name → `LocalPackage { version, checksum, path }`, for workspace-member packages — currently
unpopulated by any CLI path since `Workspace` isn't wired in). `pata/cli`'s adapter writes path
dependencies into the `dependencies` map (not `local_packages`) with `path: Some(...)` set
(`to_package_dependency`, `project.rs:222-227`) — `local_packages` is dead from the CLI's side, same
as `Workspace` itself.

## Cross-references

- [implementation-status.md](implementation-status.md#phase-ii--synthesis-lsp-wasm-tooling) — phase
  checklist entry this doc supersedes with detail.
- [docs/spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) — `[tegemezi]`
  manifest section and `pata` command table (user-facing).
- [docs/spec/02-architecture-and-files.md](../spec/02-architecture-and-files.md) — project directory
  layout, `pata/package/`'s stated role ("Manages `pata.toml` and generates `pata.lock`").
- [mwalimu-design.md](mwalimu-design.md) — sibling design doc this one follows in structure/style.
