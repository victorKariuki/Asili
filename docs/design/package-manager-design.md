# Package manager (Pakiti/Moduli) design

The Asili package manager is split across two crates: `pata/package` (the `pata-package`
library — dependency spec, lockfile, resolver, registry, fetch primitives) and `pata/cli`'s
[`pipeline/project.rs`](../../pata/cli/src/pipeline/project.rs) and
[`pipeline/resolve.rs`](../../pata/cli/src/pipeline/resolve.rs) (the CLI-facing layer that parses
`pata.toml`, drives `pata jenga`/`pata ongeza`/`pata ondoa`, and turns `leta <name>` imports into
loaded/merged ASTs). This doc covers the current, unified architecture — superseding both the
two-bullet summary in [implementation-status.md](implementation-status.md#phase-ii--synthesis-lsp-wasm-tooling)
and this doc's own earlier "adapter approach" write-up (kept only as history, below).

## One manifest format: `pata.toml`, real TOML, single/leaf or workspace-root

Every Asili project — single package or workspace root — has exactly one manifest file,
`pata.toml`, with Swahili section/key names: `[jumla]` (`jina`/`toleo`/`asili`), `[chanzo]`
(`kuingia`), `[tegemezi]`, `[jenga]` (`lengo`), and — for a workspace root — `[eneo-kazi]`
(`wanachama`, a list of member directory paths, each with its own `pata.toml`).

`load_project_config` (`pata/cli/src/pipeline/project.rs`) parses this with the real `toml` crate
(`toml::Table`), not a hand-rolled line scanner — that was the previous implementation, replaced
because it could not represent nested tables at all (needed for `[eneo-kazi]`'s `wanachama`
array) and silently skipped genuinely malformed TOML instead of erroring. The Swahili section/key
*names* read are unchanged from before, so every existing hand-written `pata.toml` fixture parses
identically; only the parsing mechanism changed.

**There is no `Asili.toml` file anymore.** An earlier design had `pata-package` define a second,
English-keyed manifest dialect (`WorkspaceConfig`/`Manifest`/`PackageMetadata`, `[package]`/
`[dependencies]`/`[workspace]` sections) in a separate `Asili.toml`, used only for workspace
topology. That second format and file were retired in favor of `pata.toml`'s own `[eneo-kazi]`
table — one manifest file, one syntax, for every project regardless of whether it's a workspace
root. `pata_package::manifest` now holds only `Dependency`/`DependencyTable` (used by the
resolver, below) — `Manifest`, `WorkspaceConfig`, `PackageMetadata`, and `pata_package::Workspace`
(`pata/package/src/workspace.rs`) were deleted outright once nothing outside their own tests
referenced them.

```rust
// A workspace root's pata.toml
[jumla]
jina = "mradi"
toleo = "0.1.0"
asili = "1.1"

[eneo-kazi]
wanachama = ["core", "lib"]
```

`find_workspace_root` (`pata/cli/src/pipeline/project.rs`) walks upward from a starting directory
looking for a `pata.toml` whose `[eneo-kazi].wanachama` is non-empty, returning a `PataWorkspace`
(`root: PathBuf`, `members: BTreeMap<String, PathBuf>` — member directory name to resolved path,
each validated to actually contain its own `pata.toml`). `pata jenga --workspace-info` and
`compile_project`'s workspace-presence check both call this. `pata njozi --workspace` scaffolds
exactly this shape: a root `pata.toml` with `[eneo-kazi]`, plus `core/` and `lib/` member
directories each with their own `pata.toml`/`src/kuu.as`.

## Module layout: `pata/package/src/`

Declared in [`lib.rs`](../../pata/package/src/lib.rs):

- **`manifest.rs`** — `Dependency` (`#[serde(untagged)]` over `Table(DependencyTable)` /
  `Version(String)`, table tried first) and `DependencyTable` (`version`/`path`/`git`/`branch`).
  `Dependency::version()`/`path()`/`git()` accessors. Used directly by `pata.toml`'s `[tegemezi]`
  parsing in `pata-cli` and by the resolver below — no adapter/conversion layer anymore.
- **`resolver.rs`** — `Resolver::resolve(root, dependencies, existing_lock) -> Result<LockFile>`.
  Real semver-range constraint solving (not verbatim-lock-whatever-is-given), against three
  dependency shapes:
  - **Path** — no versions to choose between, unchanged, and not walked transitively (a path
    dependency's own deps are resolved as part of `pata_core`'s module resolver, not here).
  - **Git** (`{ git = "...", version = "^1.2" }`) — the vendored copy under
    `.asili/packages/<name>/` is the only "available version"; its `.pata-version` marker
    (written by `pata ongeza --git`) says what was actually fetched, checked against the
    constraint. Not walked transitively either — no manifest format exists yet for a vendored
    git tree to declare its own deps.
  - **Registry** (a bare version string) — resolved against a real `LocalRegistry` index at
    `<root>/.asili/registry/`; every version's `RegistrySource` (git or path) is fetched for real
    into `.asili/packages/<name>/` the first time it's picked, then content-hashed via
    `fetch::hash_dir`. **Transitive**: a resolved `RegistryEntry`'s own declared `deps:
    Vec<RegistryDep>` are queued and resolved in turn (breadth-first), so a registry package's
    own registry dependencies are fetched and locked too — not just what a project's own
    `[tegemezi]` names directly. Every constraint seen for a given package name (from any direct
    or transitive dependent) accumulates; a version must satisfy all of them at once or
    resolution fails with a named conflict, rather than one dependent's requirement silently
    overwriting another's lock entry.
  `existing_lock` is honored for real: an already-locked version that still satisfies the current
  constraint set stays locked (avoids lockfile churn / unnecessary re-fetching) rather than being
  re-resolved on every build.
- **`registry.rs`** — `LocalRegistry`: a minimal, self-hostable index, one JSON file per package
  (`<index_root>/<name>.json` holding every published, non-yanked version as a `RegistryEntry`
  with a real fetchable `source`). Modeled on Cargo's alternative-registry RFC (JSON index +
  fetchable location per version, no API server required).
- **`fetch.rs`** — `fetch_git` (real `git2`-based clone, strips `.git/`, returns a content hash)
  and `hash_dir` (deterministic SHA-256 over every file under a directory, recursively — the one
  real content-integrity primitive everything else builds on).
- **`lock.rs`** — `LockFile`/`LockedDependency`/`LocalPackage`, real TOML load/save (`pata.lock`
  on disk: `version`, `locked_at` RFC3339 timestamp, `dependencies` map, `local_packages` map),
  `workspace_checksum()` (SHA-256 over all locked checksums), and
  `verify_content_integrity(root)` — re-hashes every vendored git/registry dependency's on-disk
  content and compares against what the lock recorded at fetch time, returning every mismatching
  dependency name. This is the actual security property a lockfile exists for: a tampered/swapped
  `.asili/packages/<name>/` directory is caught before compiling, not silently trusted. Called
  from `pata-cli`'s `compile_project` via `verify_lockfile_integrity` (`project.rs`) before every
  build; a mismatch is a hard build error naming the affected dependency and its expected/actual
  hash.
- **`paths.rs`** — `Paths::new(root)` computes the fixed directory layout: `.asili/`
  (package-manager state), `.asili/packages/` (vendored external packages), `.asili/registry/`
  (local registry index), `kilele/` (build artifacts), `lib/`, `src/`. `package_path(name)` —
  used by the resolver and by `verify_content_integrity` to find a vendored dependency's root.
- **`constraints.rs`** — `VersionConstraint`, a thin wrapper over `semver::VersionReq`; the
  resolver itself uses `semver::VersionReq` directly rather than this type.
- **`lib.rs`** — re-exports: `Dependency`, `DependencyTable`, `IntegrityMismatch`, `LockFile`,
  `LockedDependency`, `Resolver`, `Paths`, `fetch_git`, `hash_dir`, `FetchError`,
  `VersionConstraint`, `PackageMetadata`, `RegistryEntry`, `RegistrySource`, `LocalRegistry`.

## How `pata/cli` delegates to `pata_package`

Two pipeline files drive this, both under `pata/cli/src/pipeline/`:

**`project.rs`** owns `pata.toml`/`pata.lock` file I/O and workspace discovery:
- `load_project_config` — parses `pata.toml` via real TOML (`toml::Table`), reading `[jumla]`,
  `[chanzo]`, `[jenga]`, `[tegemezi]`, and `[eneo-kazi]` directly into `ProjectConfig` — no
  intermediate adapter type.
- `find_workspace_root`/`PataWorkspace` — see above.
- `write_lockfile` — calls `pata_package::Resolver::resolve(root, &pkg_deps,
  existing.as_ref())` directly (`cfg.dependencies` is already `pata_package::Dependency`-shaped,
  no conversion needed), and — as a CLI-level behavior not present in `pata_package` itself —
  preserves the previous `locked_at` timestamp when the resolved dependency set is unchanged, so
  re-running `pata jenga`/`pata ongeza` without dependency changes doesn't churn `pata.lock`.
- `read_lockfile` — loads `pata.lock` via `pata_package::LockFile::load` and converts each
  `LockedDependency` back to a `Dependency` (`path` present → `Path`; `source == "git"` →
  `Git`; otherwise → `Version`).
- `verify_lockfile_integrity` — loads the raw `LockFile` and calls
  `LockFile::verify_content_integrity(root)`; returns an empty list (not an error) when there's
  no lockfile yet, so a fresh single-file build with nothing to verify isn't penalized.

**`resolve.rs`** owns module resolution (turning `leta <name>` imports into loaded/merged ASTs)
and is where locked dependencies actually get used to find source files:
- `find_module_file` checks, in order: path dependencies (`<path>/src/<name>.as`); then, for a
  `Version`/`Git` entry, the vendored package cache via `pata_package::Paths::new(root)
  .package_path(name)` joined with `src/<name>.as`; then local `<root>/<name>.as`,
  `<root>/lib/<name>.as`, `<root>/lib/<name>/mod.as`, and stdlib `<root>/lib/std/<name>.asi`.
- `compile_project` (`pipeline/compile.rs`) calls `load_project_config`, overwrites
  `cfg.dependencies` with `read_lockfile(root)`'s result when `pata.lock` exists (so a build with
  a lockfile present resolves against the **locked** versions/paths, mirroring Cargo's
  `Cargo.lock` precedence), then calls `verify_lockfile_integrity` and hard-fails the build on any
  content mismatch before compilation proceeds.
- `resolve_all`/`resolve_one` do the actual recursive import walk (cycle detection via `RES001`,
  missing-module via `RES002`) and call `merge_for_eval`, which delegates to
  `asili_parser::merge_modules` (`core/parser/src/module_merge.rs`) — shared with `driver/wasm`'s
  in-memory bundler so the merge rules live in exactly one place.

`pata ongeza` (`pata/cli/src/commands/ongeza.rs`) ties it together at the command level:
- No source: `update_dependency` (writes a bare version string into `[tegemezi]`) →
  `load_project_config` → `write_lockfile`, which resolves against the local registry index.
- `--git <url>`: `update_dependency_git` (writes `{ git = "...", version = "..." }`) → real
  `pata_package::fetch_git` clone into `.asili/packages/<name>/` (strips `.git/`, computes a real
  SHA-256 content hash, writes a `.pata-version` marker) → `write_lockfile`.

`pata ondoa <lib>` is the counterpart: removes the `[tegemezi]` entry and re-resolves the
lockfile without it.

## What actually works vs. what's explicitly not implemented

**Works:**
- **Path dependencies** — resolved directly against the filesystem path, no lockfile/vendoring
  involved.
- **Git dependencies** (`pata ongeza --git`) — real clone, real content hash, verified against
  `pata.lock` at every subsequent build.
- **Registry dependencies** — resolved against a real local, file-based index
  (`.asili/registry/`), fetched and content-hashed on first use.
- **Transitive registry dependencies** — a package's own declared deps are fetched and locked
  too, with real conflict detection when two dependents require incompatible ranges of the same
  transitive package.
- **Lockfile generation, reuse, and integrity verification** — `pata.lock` is real TOML, written
  deterministically, consulted by `compile_project` in preference to `pata.toml`'s live
  `[tegemezi]` when present, and re-verified (content hash, not just presence) before every build.
- **Unified single-manifest workspaces** — `pata.toml`'s `[eneo-kazi]` table, discovered by
  `find_workspace_root`, driving `pata jenga --workspace-info` and `pata njozi --workspace`.

**Not implemented — confirmed directly in the code, not inferred:**
- **No hosted/remote registry** — `LocalRegistry` is file-based (`.asili/registry/` on disk), not
  a network service; there's no HTTP client anywhere in `pata-package`'s dependencies. This
  matches the Zig precedent this project's own production-readiness doc cites: content-hash-
  pinned git/local sources are a legitimate floor without a central hosted index.
- **`pata ongeza` is not workspace-aware** — it still resolves against the single project's own
  `pata.toml`, not a workspace root's aggregate view. Adding a member to a workspace still means
  running `pata ongeza` inside that member's own directory.
- **No full ABI/type-stability check tied to registry publishing** — `pata thibitisha`'s
  type-stability check (see [implementation-status.md](implementation-status.md)) uses a git-tag
  baseline, independent of this registry.

## Two real bugs a prior rewire fixed (kept as history)

**(a) Public module constants were never exported.** `Constant` (`core/parser/src/ast.rs`) has no
`is_public` field — there's no visibility modifier on constants in the language, so every one is
treated as exported. The bug was that `build_export_table` and `merge_modules`'s constant-handling
arm didn't consistently export every module-level `thabiti` constant. Both are now unconditional.
Covered by `core/evaluator/tests/module_constants.rs`.

**(b) `merge_for_eval`/`merge_for_semantic` only merged imported functions, not
structs/traits/impls.** `asili_parser::merge_modules` now merges `functions`, `constants`,
`structs`, `traits`, and `impls` — impls merged unconditionally by target-type dedup (no
visibility check, since an impl block has no name of its own to select on).

## Checksums and the lockfile

Every fetchable dependency (git, registry) gets a **real SHA-256 content hash** via
`fetch::hash_dir` — over the actual fetched file tree, not a hash of the `name@version` string.
Path dependencies (which have no fetchable content of their own — they're live local source) use
`compute_checksum(name, version)` in `resolver.rs`, a hash of the name/version string, kept only
because path dependencies need *some* checksum value to satisfy `LockedDependency`'s schema, not
because it's a meaningful integrity check.

The lockfile itself (`pata/package/src/lock.rs`) is TOML: `version` (schema version string,
currently `"1"`), `locked_at` (RFC3339 timestamp via `chrono`), `dependencies` (map of name →
`LockedDependency { version, checksum, path: Option<String>, source }`), and `local_packages`
(map of name → `LocalPackage`, currently unpopulated — no CLI path writes to it; it predates the
current `[eneo-kazi]` workspace design and nothing produces `LocalPackage` entries today).
`LockFile::verify_content_integrity(root)` is what turns the recorded checksum into an actual
security property: it re-hashes every vendored git/registry dependency's on-disk content right
before a build and compares against what's in `pata.lock`, catching a swapped or hand-edited
`.asili/packages/<name>/` directory that a checksum which is only ever *written*, never
*re-checked*, could not detect.

## Cross-references

- [implementation-status.md](implementation-status.md#phase-ii--synthesis-lsp-wasm-tooling) —
  phase checklist entry this doc supersedes with detail.
- [pata-production-readiness.md](pata-production-readiness.md) — the floor/stretch tracking doc
  this design backs; item 1 (fetch/verify) and the transitive-resolver stretch item are both
  implemented per this doc's description above.
- [docs/spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) — `[tegemezi]`
  manifest section and `pata` command table (user-facing).
- [docs/spec/02-architecture-and-files.md](../spec/02-architecture-and-files.md) — project
  directory layout, `pata/package/`'s stated role.
- [mwalimu-design.md](mwalimu-design.md) — sibling design doc this one follows in structure/style.
