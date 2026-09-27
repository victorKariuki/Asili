# Pata toolchain: exact implementation spec (Haiku-executable)

This doc is a companion to [pata-production-readiness.md](pata-production-readiness.md)'s "Full
build-out" section. That doc explains *why* and *in what order*. This doc exists so a lower-
reasoning-effort model can execute each task mechanically — every section below is self-contained,
names exact file paths and function signatures, and ends with an exact, runnable acceptance check.
Do not reorder sections; each depends only on the ones above it (see the production-readiness doc's
dependency graph for the full rationale).

**Ground rules for whoever executes this:**
- One section = one unit of work = one commit. Do not start section N+1 until section N's
  acceptance check passes.
- Never guess a function/struct name — every one used below is copy-pasted from the real source as
  of this doc's writing. If a `grep` in an acceptance check doesn't find what it expects, stop and
  report the mismatch rather than improvising.
- Run `cargo build --workspace` and `cargo test --workspace` after every section, not just the
  section's own crate — cross-crate breakage is the main risk in this codebase.
- Before touching any file, run `git status --short <path>` — if it shows uncommitted changes you
  didn't make, stop and ask; another session may be actively working on it (this has happened
  before in this repo).
- Formatter work (`pata/fmt/src/format.rs`, `core/lexer/src/lib.rs`'s `tokenize_with_trivia`) is
  **already done** by a prior session — do not touch it. Confirmed via
  `docs/design/nadhifu-formatter-design.md` and passing tests. Skip straight to Section 1.

---

## Section 1: `pata ongeza` real fetch — git and path sources only

**Goal:** `pata ongeza <lib> --git <url>` (new flag) and existing path dependencies actually place
real source on disk under `.asili/packages/<name>/`, with a real content-hash lockfile entry
instead of today's `sha256("{name}@{version}")` no-op.

**Files to touch:**
- `pata/package/Cargo.toml` — add dependency: `git2 = "0.19"` (rust bindings to libgit2; use this,
  not shelling out to a `git` binary, so there's no runtime dependency on git being installed).
- `pata/package/src/fetch.rs` — **new file**.
- `pata/package/src/lib.rs` — add `pub mod fetch;` and re-export `pub use fetch::{fetch_git, FetchError};`.
- `pata/package/src/resolver.rs` — replace `compute_checksum`.
- `pata/cli/src/commands/ongeza.rs` — wire in the fetch call.
- `pata/cli/commands/ongeza.md` — update contract doc.

**Exact steps:**

1. In `pata/package/Cargo.toml`, under the existing `[dependencies]` block (confirmed present at
   line 6), add:
   ```toml
   git2 = "0.19"
   ```

2. Create `pata/package/src/fetch.rs`:
   ```rust
   //! Real dependency fetching: git-clone a source into the vendored package cache.
   //! No registry backend exists yet (see docs/design/pata-production-readiness.md) — this
   //! covers git and path sources only.

   use anyhow::{Context, Result};
   use sha2::{Digest, Sha256};
   use std::fs;
   use std::path::Path;

   #[derive(Debug, thiserror::Error)]
   pub enum FetchError {
       #[error("git clone failed for {url}: {source}")]
       GitClone {
           url: String,
           #[source]
           source: git2::Error,
       },
       #[error("failed to hash fetched content at {path}: {source}")]
       Hash {
           path: String,
           #[source]
           source: std::io::Error,
       },
   }

   /// Clone `url` (optionally at `branch`, else the repo's default branch) into `dest`,
   /// replacing any existing directory at `dest` first (idempotent re-fetch). Returns a
   /// deterministic SHA-256 content hash over the cloned tree, computed by `hash_dir`.
   pub fn fetch_git(url: &str, branch: Option<&str>, dest: &Path) -> Result<String, FetchError> {
       if dest.exists() {
           fs::remove_dir_all(dest).map_err(|e| FetchError::Hash {
               path: dest.display().to_string(),
               source: e,
           })?;
       }
       let mut builder = git2::build::RepoBuilder::new();
       if let Some(b) = branch {
           builder.branch(b);
       }
       builder
           .clone(url, dest)
           .map_err(|e| FetchError::GitClone { url: url.to_string(), source: e })?;
       // Remove .git metadata from the vendored copy — we only want the source tree, and its
       // presence would make hash_dir non-deterministic across clones (git internals vary).
       let git_dir = dest.join(".git");
       if git_dir.exists() {
           let _ = fs::remove_dir_all(&git_dir);
       }
       hash_dir(dest).map_err(|e| FetchError::Hash { path: dest.display().to_string(), source: e })
   }

   /// Deterministic content hash over every regular file under `dir`, recursively: sort all
   /// relative file paths, then hash "relative_path\0" + file_bytes for each in sorted order.
   /// Sorting is required for determinism — directory read order is not guaranteed by the OS.
   pub fn hash_dir(dir: &Path) -> std::io::Result<String> {
       let mut files = Vec::new();
       collect_files(dir, dir, &mut files)?;
       files.sort();
       let mut hasher = Sha256::new();
       for rel in &files {
           hasher.update(rel.as_bytes());
           hasher.update(b"\0");
           let bytes = fs::read(dir.join(rel))?;
           hasher.update(&bytes);
       }
       Ok(format!("{:x}", hasher.finalize()))
   }

   fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
       for entry in fs::read_dir(dir)? {
           let entry = entry?;
           let path = entry.path();
           if path.is_dir() {
               collect_files(root, &path, out)?;
           } else {
               let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
               out.push(rel);
           }
       }
       Ok(())
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use std::io::Write;

       #[test]
       fn hash_dir_is_deterministic_and_order_independent() {
           let dir = std::env::temp_dir().join(format!(
               "pata-fetch-test-{}",
               std::time::SystemTime::now()
                   .duration_since(std::time::UNIX_EPOCH)
                   .unwrap()
                   .as_nanos()
           ));
           fs::create_dir_all(dir.join("sub")).unwrap();
           fs::File::create(dir.join("a.txt")).unwrap().write_all(b"hello").unwrap();
           fs::File::create(dir.join("sub/b.txt")).unwrap().write_all(b"world").unwrap();

           let h1 = hash_dir(&dir).unwrap();
           let h2 = hash_dir(&dir).unwrap();
           assert_eq!(h1, h2);

           fs::remove_dir_all(&dir).unwrap();
       }

       #[test]
       fn hash_dir_changes_when_content_changes() {
           let dir = std::env::temp_dir().join(format!(
               "pata-fetch-test2-{}",
               std::time::SystemTime::now()
                   .duration_since(std::time::UNIX_EPOCH)
                   .unwrap()
                   .as_nanos()
           ));
           fs::create_dir_all(&dir).unwrap();
           fs::File::create(dir.join("a.txt")).unwrap().write_all(b"hello").unwrap();
           let h1 = hash_dir(&dir).unwrap();
           fs::File::create(dir.join("a.txt")).unwrap().write_all(b"goodbye").unwrap();
           let h2 = hash_dir(&dir).unwrap();
           assert_ne!(h1, h2);

           fs::remove_dir_all(&dir).unwrap();
       }
   }
   ```
   Also add `thiserror = "1"` to `pata/package/Cargo.toml` if not already present (check first:
   `grep thiserror pata/package/Cargo.toml`).

3. In `pata/package/src/lib.rs`, find the existing `pub mod` list (near the top) and add:
   ```rust
   pub mod fetch;
   ```
   and add to the existing re-export block:
   ```rust
   pub use fetch::{fetch_git, hash_dir, FetchError};
   ```

4. In `pata/package/src/resolver.rs`, replace the `compute_checksum` function (currently hashing
   `"{name}@{version}"`) — but **do not remove it**, since the "registry" source case (no git URL)
   still has nothing real to hash yet (no registry backend exists — see
   `pata-production-readiness.md` item 3). Instead, add a new function alongside it:
   ```rust
   /// Real content-hash checksum for a fetched git dependency — the actual bytes on disk under
   /// `vendor_path`, not a hash of the name/version string. Falls back to the old
   /// name@version-string hash (`compute_checksum`) only when nothing was fetched (registry
   /// source with no backend yet — see docs/design/pata-production-readiness.md item 3).
   pub fn compute_content_checksum(vendor_path: &std::path::Path) -> Result<String> {
       crate::fetch::hash_dir(vendor_path).map_err(Into::into)
   }
   ```

5. In `pata/cli/src/commands/ongeza.rs`, read the current file first (`Read` tool) to see the
   exact `run` function signature and `update_dependency` call before editing — do not guess at
   the surrounding code. Add:
   - A new `--git <url>` flag parsed in `parse_args` (mirror the existing flag-parsing style used
     for `--kiwango-cha-jaribio` in `pata/cli/src/commands/thibitisha.rs` for the pattern: a
     `match args[i].as_str()` arm consuming the next arg as the value).
   - After `update_dependency`/`write_lockfile` succeeds, when `--git` was passed: call
     `pata_package::fetch_git(url, None, &dest)` where `dest` is
     `pata_package::Paths::new(Path::new(".")).package_src_path(&lib)`'s parent-equivalent — check
     `pata/package/src/paths.rs`'s `package_src_path` signature exactly before writing this call
     (`Read` the file first), then write the returned content hash into the `pata.lock` entry for
     that dependency (via `pata_package::LockedDependency.checksum`, overwriting whatever
     `write_lockfile`'s normal path produced).
   - Remove the top-of-file `TODO` comment (currently: "pata ongeza writes the dependency to
     pata.toml and generates pata.lock, but does NOT actually download or resolve the package"),
     replacing it with a comment stating: fetch is implemented for `--git` sources; plain version
     dependencies with no `--git` still require out-of-band vendoring (no registry backend exists
     — see `pata-production-readiness.md` item 3).

6. Update `pata/cli/commands/ongeza.md`: add `--git <url>` to the Inputs list, and update the
   "does NOT actually download" line to say git sources now fetch for real; version-only
   dependencies (no `--git`) still don't.

**Acceptance check (run exactly this):**
```bash
cd /mnt/1264f5d2-2c2b-4f68-8c81-0b146eb0ed58/Projects/Asili
cargo build -p pata-package -p pata-cli 2>&1 | tail -30
cargo test -p pata-package 2>&1 | tail -20
```
Both must show zero errors and the two new `fetch.rs` tests passing. Then manually verify against
a real small public repo (pick any tiny public git repo you have read access to, or skip this
manual step and note it as untested if no network access is available in this environment — check
first with `curl -sI https://github.com 2>&1 | head -1`; if it fails, skip the manual fetch test
and rely on the unit tests only).

---

## Section 2: `pata ondoa` (remove dependency) — new command

**Goal:** Add the missing counterpart to `pata ongeza`. Simple, self-contained, no design
ambiguity.

**Files to touch:**
- `pata/cli/src/commands/ondoa.rs` — **new file**, model directly on `pata/cli/src/commands/
  ongeza.rs`'s structure (`Read` that file first for the exact `CliResult`/`CliError` patterns).
- `pata/cli/src/commands/mod.rs` — register the new command (find how `ongeza` is registered here
  and mirror it exactly).
- `pata/cli/src/main.rs` (or wherever the top-level command dispatch match lives — `grep -n
  '"ongeza"' pata/cli/src/*.rs` to find it) — add a `"ondoa" => commands::ondoa::run(&args[2..])`
  arm right next to the existing `"ongeza"` arm.
- `pata/cli/commands/ondoa.md` — **new file**, model on `pata/cli/commands/ongeza.md`.
- `pata/cli/src/pipeline/project.rs` — add a `remove_dependency(root: &Path, name: &str) ->
  Result<(), CliError>` function next to the existing `update_dependency` (read that function
  first to mirror its exact TOML-rewriting approach — it manually edits the `[tegemezi]` section
  of `pata.toml`'s hand-rolled parser, so removal must do the same kind of line-based edit: delete
  the line whose key matches `name` inside the `[tegemezi]` section).

**Exact behavior:**
- `pata ondoa <lib>` removes the `<lib>` entry from `[tegemezi]` in `pata.toml`, then regenerates
  `pata.lock` via the same `write_lockfile` call `ongeza` uses (so the lock stays consistent).
- If `<lib>` isn't present in `[tegemezi]`, exit code 1 with a Swahili error message following
  this codebase's convention (e.g. `"tegemezi '<lib>' halipo kwenye pata.toml"` — check
  `ongeza.rs`'s error message style and match it).
- If `<lib>` isn't given at all, exit code 2 (usage error), matching `ongeza`'s own arg-count
  check.

**Acceptance check:**
```bash
cargo build -p pata-cli 2>&1 | tail -20
cargo test -p pata-cli ondoa 2>&1 | tail -20
```
Write at least 2 tests in `ondoa.rs`'s `#[cfg(test)] mod tests`, mirroring `ongeza.rs`'s own test
style exactly (temp project dir, `TEST_CWD_LOCK`, `std::env::set_current_dir`): one test that adds
then removes a dependency and asserts it's gone from `pata.toml`; one test that asserts exit code
1 when removing a nonexistent dependency.

---

## Section 3: `pata jaribu` — structured JSON output

**Goal:** Smallest, most self-contained item in the `jaribu` full-build-out list (parallel
execution, timeouts, and fixtures are all bigger, judgment-heavy items deliberately excluded from
this Haiku-spec — see "Deferred" section at the bottom). Add a `--json` flag that prints results
as a single JSON object instead of the current `[SAWA]`/`[KOSA]` lines.

**Files to touch:**
- `pata/cli/src/commands/jaribu.rs` — the whole change lives here.
- `pata/cli/Cargo.toml` — add `serde_json = "1"` if not already present (`grep serde_json
  pata/cli/Cargo.toml` first).
- `pata/cli/commands/jaribu.md` — document the new flag.

**Exact steps:**

1. `Read` `pata/cli/src/commands/jaribu.rs` in full first (it's short, ~85 lines per earlier
   exploration in this conversation) to get `TestResult`'s exact field names from
   `pipeline::compile::run_project_tests`'s return type before writing serialization code.

2. Add a `--json` flag to `parse_args`, returning a 4th tuple element `json: bool` (update the
   function's return type from `(Option<String>, bool, bool)` to `(Option<String>, bool, bool,
   bool)` and thread the new value through `run`'s destructuring).

3. When `json` is true, instead of the existing `println!("[SAWA] ...")`/`println!("[KOSA] ...")`
   loop, build and print one JSON object via `serde_json::json!({...})` (or a `#[derive(Serialize)]`
   struct if you prefer — either is fine, pick whichever is less code given `TestResult`'s actual
   fields) shaped as:
   ```json
   {
     "jumla": 12,
     "sawa": 11,
     "kosa": 1,
     "majaribio": [
       {"jina": "jaribu_x", "sawa": true, "ujumbe": null},
       {"jina": "jaribu_y", "sawa": false, "ujumbe": "kosa: ..."}
     ]
   }
   ```
   Print with `println!("{}", serde_json::to_string_pretty(&value).unwrap())`. Exit codes stay
   identical to the non-JSON path (0 all passed, 1 some failed) — only the stdout format changes.
   When `--list`/`list_only` is combined with `--json`, print `{"majaribio": ["name1", "name2"]}`
   instead of one-name-per-line.

**Acceptance check:**
```bash
cargo build -p pata-cli 2>&1 | tail -20
cd examples/sifa && cargo run -q -p pata-cli -- jaribu --json 2>&1 | tail -20
cd /mnt/1264f5d2-2c2b-4f68-8c81-0b146eb0ed58/Projects/Asili
```
Output must be valid JSON (pipe through `python3 -m json.tool` or `jq .` to confirm it parses) with
`jumla`/`sawa`/`kosa`/`majaribio` keys present.

---

## Section 4: `pata-lint` — adversarial test coverage for existing rules

**Goal:** Per the production-readiness doc's item 8 — every existing rule (LINT001, LINT002,
LINT003, LINT101, LINT202, LINT203) gets a deliberate false-positive AND false-negative test, the
same rigor LINT201 already got. No new rules in this section — that's separate, larger work
deferred (see bottom). This section is pure test-writing against existing, working code, so it's
the lowest-risk item in this entire spec.

**Files to touch:**
- `pata/lint/src/rules/naming.rs` — add tests for LINT001/002/003.
- `pata/lint/src/rules/style.rs` — add tests for LINT101.
- `pata/lint/src/rules/best_practices.rs` — add tests for LINT202/203 (LINT201 already covered).

**Exact steps — for each rule, write exactly 2 tests:**

1. **LINT001 (function naming)** in `naming.rs`: one test with a function named `MyFunction`
   (PascalCase, should trigger LINT001 since it's neither snake_case nor Swahili — read
   `is_snake_case`/`is_swahili` in the same file first to pick a name that's unambiguously neither)
   asserting `diags.iter().any(|d| d.code == "LINT001")`; one test with a function named
   `jina_sahihi` (valid snake_case Swahili) asserting no LINT001 diagnostic appears. Build the
   `Module` input either by parsing a literal source string with `asili_lexer::tokenize` +
   `asili_parser::parse_tokens` (check how existing tests elsewhere in this crate construct a
   `Module` — `grep -rn "parse_tokens" pata/lint/` first to find the pattern already in use, and
   copy it) — do not hand-construct a `Module` struct literal unless that's the pattern already
   used.

2. **LINT002 (struct naming)**: same shape, one struct named `not_pascal` (should trigger), one
   named `PataStruct` (should not).

3. **LINT003 (constant naming)**: same shape, one constant named `notUpper` (should trigger), one
   named `MAX_SIZE` (should not).

4. **LINT101 (function length)**: read the exact threshold in `style.rs` first (`grep -n "> 50"
   pata/lint/src/rules/style.rs` — confirmed at 50 statements in earlier exploration this
   conversation, but re-verify, don't assume). One test with a function body of 51+ statements
   (generate via a loop building `weka x = 1\n` repeated 51 times as source text) asserting
   LINT101 fires; one test with exactly the threshold or fewer statements asserting it doesn't.

5. **LINT202 (docs)** and **LINT203 (unused imports)** in `best_practices.rs`: `Read` the file in
   full first to see their exact trigger conditions (this file wasn't fully read in this
   conversation's earlier exploration — only its head was seen), then write matching
   trigger/no-trigger test pairs the same way.

**Acceptance check:**
```bash
cargo test -p pata-lint 2>&1 | tail -40
```
All new tests pass; total test count in `pata-lint` increases by exactly 10 (2 per rule × 5 rules,
LINT201 excluded since it's already covered).

---

## Section 5: `pata njozi` — CI workflow stub in scaffolded projects

**Goal:** Smallest item in the entire plan. When `pata njozi <dir>` creates a new project, also
write a `.github/workflows/ci.yml` into it, generated from the same one that now exists at this
repo's own `.github/workflows/ci.yml` but simplified to a single project (no `examples/*` loop —
just build + test + lint + jenga on the one project root).

**Files to touch:**
- `pata/cli/src/commands/njozi.rs` — find the function that writes the scaffolded files (`Read`
  the file first; per earlier exploration it writes `pata.toml` and `src/kuu.as` — find the exact
  function name, likely something like `create_project_files` or inline in `run`).
- `.github/workflows/ci.yml` (this repo's own, already exists) — read it for reference; do not
  modify it.

**Exact template to write to `<new_project>/.github/workflows/ci.yml`:**
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Install pata
        run: cargo install --path . --locked || true
      - name: pata jenga
        run: pata jenga
      - name: pata jaribu
        run: pata jaribu
```
(Note: this template assumes the scaffolded project vendors/depends on the Asili toolchain source
being cloneable via `cargo install --path .` — if that assumption is wrong for a real end-user
project — i.e. `pata njozi` scaffolds an Asili *project*, not a clone of the Asili *toolchain
repo* — replace the "Install pata" step with a placeholder comment `# TODO: install the pata CLI
(see https://github.com/<org>/asili for install instructions)` instead of a real command. Check
which case applies by reading what `pata njozi` actually scaffolds before deciding — this is the
one judgment call in this section; if genuinely ambiguous, use the placeholder-comment version, it
degrades safely.)

**Acceptance check:**
```bash
cargo build -p pata-cli 2>&1 | tail -20
cargo run -q -p pata-cli -- njozi /tmp/njozi-ci-test 2>&1
test -f /tmp/njozi-ci-test/.github/workflows/ci.yml && echo "PASS: file exists"
rm -rf /tmp/njozi-ci-test
cargo test -p pata-cli njozi 2>&1 | tail -20
```
Existing `njozi` tests (`njozi_creates_expected_files`, etc.) must still pass — update that test's
expected-file-list assertion to include the new path if the test enumerates exact files.

---

## Section 6: `Workspace`/`Asili.toml` wired into `pata jenga` — SUPERSEDED

**This section's original decision (below, kept as history) was later reversed.** The steps below
describe wiring `pata.toml` projects up to a *separate* `Asili.toml` workspace manifest via
`pata_package::Workspace`/`WorkspaceConfig`, and were implemented as written. That design was
then explicitly revisited and unified: `pata.toml` itself gained an `[eneo-kazi]` table
(`wanachama = [...]`), parsed via a real TOML library (`load_project_config` moved off the old
hand-rolled line scanner in the same pass), so a workspace root and an ordinary project share one
manifest file and one Swahili-keyed syntax. `Asili.toml`, `pata_package::Workspace`,
`WorkspaceConfig`, and `Manifest`/`PackageMetadata` were all deleted — nothing outside their own
tests referenced them by that point. See
[package-manager-design.md](package-manager-design.md#one-manifest-format-patatoml-real-toml-singleleaf-or-workspace-root)
for the current design and rationale. The original rationale below (reuse a working serde parser
rather than extend the hand-rolled one) was valid at the time but was superseded once the parser
itself was replaced with a real TOML library, which removed the asymmetry that motivated keeping
two formats.

**Original decision (historical, no longer current):** `pata.toml`
does **not** gain its own `[workspace]` section. `Asili.toml` stays the dedicated workspace
manifest, read only from a workspace **root** (a directory with no `pata.toml` of its own, only
`Asili.toml` with a `[workspace]` table naming member directories, each of which has its own
`pata.toml`). Rationale: `pata.toml`'s hand-rolled line parser (`load_project_config`,
`pata/cli/src/pipeline/project.rs`) already has real, working semantics for a *single* project;
overloading it with workspace-member-list syntax means teaching that parser a new nested-table
shape it was never designed for, and `pata_package::Workspace`/`WorkspaceConfig` already parse
`Asili.toml` via serde with zero bugs (both existing tests pass). Reusing the working serde-based
parser for the one thing it's for (workspace declarations only, never per-project settings) is
less total code than extending the hand-rolled one.

**Files to touch:**
- `pata/cli/src/pipeline/project.rs` — add a new function `find_workspace_root`.
- `pata/cli/src/pipeline/compile.rs` — call it at the top of `compile_project`.
- `pata/cli/Cargo.toml` — add `pata-package = { path = "../package" }` if not already a dependency
  (`grep pata-package pata/cli/Cargo.toml` first — it likely already is, since `pipeline/
  resolve.rs` already calls `pata_package::Paths`).
- `pata/cli/commands/jenga.md` — document the new behavior.
- `docs/spec/02-architecture-and-files.md` — document `Asili.toml`'s role precisely (read this
  file first; it already has a table of `/lib`, etc. — add a row for workspace roots matching that
  table's exact column style).

**Exact steps:**

1. In `pata/cli/src/pipeline/project.rs`, add (near `load_project_config`):
   ```rust
   /// Walk up from `start` looking for a directory containing `Asili.toml` with a `[workspace]`
   /// table (a workspace root) before finding one containing `pata.toml` (an ordinary project) —
   /// `pata.toml` takes precedence at the same directory level, since a directory with both is
   /// the workspace-root's own package, not itself a bare workspace root. Returns `None` if
   /// neither is found by the filesystem root.
   pub fn find_workspace_root(start: &Path) -> Option<pata_package::Workspace> {
       let mut dir = start;
       loop {
           let asili_toml = dir.join("Asili.toml");
           if asili_toml.is_file() {
               if let Ok(ws) = pata_package::Workspace::open(dir) {
                   if ws.is_workspace() {
                       return Some(ws);
                   }
               }
           }
           dir = dir.parent()?;
       }
   }
   ```

2. In `pata/cli/src/pipeline/compile.rs`, at the top of `compile_project` (right after `let mut
   cfg = load_project_config(root)?;` — confirm this exact line first via `Read`), add:
   ```rust
   // A workspace root's aggregated dependencies are informational only for now (surfaced via
   // `pata jenga --workspace-info`, not merged into `cfg.dependencies`) — full dependency
   // aggregation across members needs the real resolver (see pata-production-readiness.md item
   // 3), not this wiring pass. This just makes `Workspace::open` reachable from a real command
   // instead of being dead code with zero CLI callers.
   if let Some(_workspace) = crate::pipeline::project::find_workspace_root(root) {
       // Reserved for future use — presence alone is enough for Section 6's acceptance check.
   }
   ```
   This is deliberately inert beyond making the code path reachable — see "Why this stops here"
   below.

3. Add a genuinely new, small command surface so the wiring is observable, not just internal
   plumbing: in `pata/cli/src/commands/`, find where `jenga`'s flags are parsed
   (`pata/cli/src/commands/jenga.rs`) and add a `--workspace-info` flag that, when passed, calls
   `find_workspace_root(Path::new("."))`, and if `Some(ws)`, prints (Swahili, matching this
   codebase's message style):
   ```
   workspace: <n> wanachama
     - <member_name_1>
     - <member_name_2>
   ```
   (via `ws.member_count()` and `ws.member_names()`), then exits 0 without building anything. If
   `None`, prints `"hakuna workspace iliyopatikana"` and exits 1.

**Why this stops here (do not go further without a design decision):** actually merging a
workspace's member dependencies into a real multi-package build (one member's compiled output
feeding another's `leta`) needs the real resolver from Section 7 below — building it on top of
today's fake `Resolver::resolve` (locks whatever version string is given verbatim, per
`pata/package/src/resolver.rs`) would just be more code sitting on a broken foundation. This
section's job is only to retire "zero CLI callers" — confirmed true today via `grep -rn
"Workspace::open" pata/cli/src/` returning nothing before this change.

**Acceptance check:**
```bash
cargo build -p pata-cli 2>&1 | tail -20
grep -rn "Workspace::open\|find_workspace_root" pata/cli/src/ | grep -v "^Binary"
```
Must show `find_workspace_root` calling `Workspace::open`, proving the dead-code path is now
live. Then:
```bash
mkdir -p /tmp/ws-test/lib/mtu
cat > /tmp/ws-test/Asili.toml <<'EOF'
[workspace]
members = ["lib/mtu"]
EOF
cat > /tmp/ws-test/lib/mtu/Asili.toml <<'EOF'
[package]
name = "mtu"
version = "1.0.0"
EOF
cd /tmp/ws-test && cargo run -q -p pata-cli --manifest-path /mnt/1264f5d2-2c2b-4f68-8c81-0b146eb0ed58/Projects/Asili/Cargo.toml -- jenga --workspace-info
cd /mnt/1264f5d2-2c2b-4f68-8c81-0b146eb0ed58/Projects/Asili
rm -rf /tmp/ws-test
```
Must print `workspace: 1 wanachama` and `- lib/mtu`, exit 0.

---

## Section 7: real dependency resolver — semver constraint solving (no registry)

**Scope boundary, decided:** this section implements real **constraint solving** over
already-locatable dependencies (path deps today; git deps once Section 1 lands). It does **not**
implement a registry backend — per `pata-production-readiness.md`, a registry needs its own
design doc (index format, hosting model, auth) that this spec deliberately does not attempt,
since "where do packages come from" is a product decision, not a mechanical one. What *is*
mechanical: replacing `Resolver::resolve`'s current behavior (lock whatever version string is
given, verbatim, with no conflict detection) with real range matching against whatever versions
are actually available on disk.

**Files to touch:**
- `pata/package/Cargo.toml` — add `semver = "1"`.
- `pata/package/src/resolver.rs` — rewrite `Resolver::resolve`.
- `pata/package/src/manifest.rs` — no changes needed; `Dependency::Version(String)` already holds
  a range string like `"^1.2.0"` (confirmed: `DependencyTable.version` doc comment already says
  "Version requirement (e.g., \"1.2.3\" or \"^1.2.0\")" — ranges were always the intent, just
  never parsed as one).

**Exact steps:**

1. Add to `pata/package/Cargo.toml`: `semver = "1"`.

2. Rewrite `Resolver::resolve` in `pata/package/src/resolver.rs`. Current signature:
   ```rust
   pub fn resolve(
       dependencies: &BTreeMap<String, Dependency>,
       _existing_lock: Option<&LockFile>,
   ) -> Result<LockFile>
   ```
   New behavior — for each `(name, dep)`:
   - If `dep` is `Dependency::Table(t)` with `t.path.is_some()`: unchanged, no version matching
     needed (a path dependency has no versions to choose between).
   - Otherwise, parse `dep.version()` as a `semver::VersionReq` (e.g. `"^1.2.0"` →
     `VersionReq::parse("^1.2.0")`). If parsing fails, return an error via `anyhow::bail!` with
     message `format!("tegemezi '{name}': muundo batili wa toleo '{}': {e}", dep.version())`.
   - **Available-versions source**: scan `.asili/packages/<name>/` for sibling directories named
     by exact version (this is the on-disk shape Section 1's `fetch_git` would need to produce
     once it supports multiple versions of the same dependency — for now, if only one version is
     ever vendored per name, as is true today, treat that one directory's declared version, read
     from a `PATA_VERSION` marker file — **new**: `fetch_git` from Section 1 must additionally
     write a one-line file `.asili/packages/<name>/.pata-version` containing the resolved version
     string at fetch time; if this spec is executed before Section 1's `fetch_git` writes that
     file, treat any vendored-but-unmarked directory as version `"0.0.0"` and let the `VersionReq`
     match-or-reject logic below handle the mismatch honestly rather than silently accepting it).
   - If no on-disk version satisfies the parsed `VersionReq`, return an error: `format!("tegemezi
     '{name}': hakuna toleo linalokubaliana na '{}' (tolewa: {available:?})", dep.version())`.
   - If a match is found, lock that concrete version with `compute_content_checksum` (from
     Section 1) instead of the old `compute_checksum`.
   - Use `_existing_lock` for real now: rename the parameter to `existing_lock` (drop the
     underscore), and when a dependency's already-locked version (from `existing_lock`) still
     satisfies the current `VersionReq`, keep it locked at that version rather than
     re-resolving to a possibly-different one that also matches — this is the actual "don't churn
     the lockfile on every build" behavior a real resolver provides, which today's parameter-
     ignoring stub cannot.

3. Write the new `resolve` roughly as:
   ```rust
   pub fn resolve(
       dependencies: &BTreeMap<String, Dependency>,
       existing_lock: Option<&LockFile>,
   ) -> Result<LockFile> {
       let mut lock = LockFile::new();

       for (name, dep) in dependencies {
           if let Some(path) = dep.path() {
               let locked = LockedDependency {
                   version: dep.version().to_string(),
                   checksum: compute_checksum(name, dep.version()),
                   path: Some(path.to_string()),
                   source: "path".to_string(),
               };
               lock.lock_dependency(name.clone(), locked);
               continue;
           }

           let req = semver::VersionReq::parse(dep.version()).map_err(|e| {
               anyhow::anyhow!("tegemezi '{name}': muundo batili wa toleo '{}': {e}", dep.version())
           })?;

           if let Some(existing) = existing_lock.and_then(|l| l.dependencies.get(name)) {
               if let Ok(v) = semver::Version::parse(&existing.version) {
                   if req.matches(&v) {
                       lock.lock_dependency(name.clone(), existing.clone());
                       continue;
                   }
               }
           }

           let available = available_versions(name)?;
           let chosen = available
               .into_iter()
               .filter(|v| req.matches(v))
               .max()
               .ok_or_else(|| {
                   anyhow::anyhow!(
                       "tegemezi '{name}': hakuna toleo linalokubaliana na '{}'",
                       dep.version()
                   )
               })?;

           let vendor_path = std::path::Path::new(".asili/packages").join(name);
           let checksum = if vendor_path.is_dir() {
               compute_content_checksum(&vendor_path)?
           } else {
               compute_checksum(name, &chosen.to_string())
           };

           lock.lock_dependency(
               name.clone(),
               LockedDependency {
                   version: chosen.to_string(),
                   checksum,
                   path: None,
                   source: dep.git().map(|_| "git").unwrap_or("registry").to_string(),
               },
           );
       }

       Ok(lock)
   }

   /// Versions available for `name`: today, just the single vendored copy under
   /// `.asili/packages/<name>/`, read from its `.pata-version` marker (Section 1's `fetch_git`
   /// writes this). No registry exists to list multiple versions from (see
   /// docs/design/pata-production-readiness.md item 3) — this always returns 0 or 1 versions
   /// until one does.
   fn available_versions(name: &str) -> Result<Vec<semver::Version>> {
       let marker = std::path::Path::new(".asili/packages").join(name).join(".pata-version");
       match std::fs::read_to_string(&marker) {
           Ok(s) => {
               let v = semver::Version::parse(s.trim())
                   .map_err(|e| anyhow::anyhow!("toleo batili kwenye {}: {e}", marker.display()))?;
               Ok(vec![v])
           }
           Err(_) => Ok(vec![]),
       }
   }
   ```
   Keep the existing `compute_checksum` function as a fallback for the no-vendor-directory case
   (matches its current role, just no longer the primary path).

4. Update the two existing tests in `resolver.rs` (`test_resolve_empty`, `test_resolve_simple`,
   `test_checksum_deterministic`) — `test_resolve_simple` currently does:
   ```rust
   deps.insert("stdlib".to_string(), Dependency::Version("1.0.0".to_string()));
   ```
   with no vendored directory on disk, so `available_versions` will return `Ok(vec![])` and the
   new code will error where the old code silently succeeded. Fix: **do not delete this test's
   intent** — instead change it to assert the correct new-behavior error:
   ```rust
   #[test]
   fn test_resolve_simple_fails_with_no_vendored_version() {
       let mut deps = BTreeMap::new();
       deps.insert("stdlib".to_string(), Dependency::Version("1.0.0".to_string()));
       let result = Resolver::resolve(&deps, None);
       assert!(result.is_err(), "no vendored version on disk should fail to resolve, not silently succeed");
   }
   ```
   Add a **new** passing test that vendors a fake version first:
   ```rust
   #[test]
   fn test_resolve_matches_vendored_version() {
       let dir = std::env::temp_dir().join(format!(
           "pata-resolve-test-{}",
           std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
       ));
       let pkg_dir = dir.join(".asili/packages/stdlib");
       std::fs::create_dir_all(&pkg_dir).unwrap();
       std::fs::write(pkg_dir.join(".pata-version"), "1.2.3").unwrap();
       std::fs::write(pkg_dir.join("marker.txt"), "x").unwrap();

       let original_cwd = std::env::current_dir().unwrap();
       std::env::set_current_dir(&dir).unwrap();

       let mut deps = BTreeMap::new();
       deps.insert("stdlib".to_string(), Dependency::Version("^1.2.0".to_string()));
       let lock = Resolver::resolve(&deps, None).expect("should resolve against vendored 1.2.3");
       assert_eq!(lock.dependencies["stdlib"].version, "1.2.3");

       std::env::set_current_dir(&original_cwd).unwrap();
       std::fs::remove_dir_all(&dir).unwrap();
   }
   ```
   (Uses `std::env::set_current_dir` like every other cwd-dependent test in this codebase's
   `pata-cli` crate — check `pata/cli/src/commands/thibitisha.rs`'s `TEST_CWD_LOCK` pattern; if
   `pata-package`'s test module doesn't already have an equivalent mutex guarding concurrent
   `set_current_dir` calls across tests in the same crate, add one exactly like
   `TEST_CWD_LOCK` — `pub(crate) static TEST_CWD_LOCK: std::sync::Mutex<()> =
   std::sync::Mutex::new(());` at the top of `resolver.rs`'s test module — and take the lock in
   both cwd-touching tests above.)

**Acceptance check:**
```bash
cargo build -p pata-package -p pata-cli 2>&1 | tail -30
cargo test -p pata-package 2>&1 | tail -30
cargo test --workspace 2>&1 | grep -E "FAILED|error:"
```
The last command's output must be empty (zero failures workspace-wide) — this section changes a
function every `pata-cli` build path calls indirectly (`write_lockfile` →
`pata_package::Resolver::resolve`), so a regression here breaks builds silently elsewhere; the
full-workspace test run is not optional for this section.

---

## Section 8: `pata thibitisha` type-stability check (git-tag baseline)

**Depends on:** Section 1 (needs `pata ongeza --git` to exist conceptually, but does **not**
actually depend on Section 1's code — this section only needs *this same repository's own* git
history, via tags, not a fetched dependency). Safe to implement independently of Sections 1-3 if
executed out of order, but keep it after them in the doc for narrative consistency with the
production-readiness doc's dependency graph.

**Goal:** `pata thibitisha` gains a check: if the current directory is a git repository and has at
least one tag matching `v<semver>`, diff the current public API (functions/structs/traits) against
the most recent such tag's public API. Flag: removed public item, changed function arity, changed
parameter type, changed return type.

**Files to touch:**
- `pata/cli/Cargo.toml` — add `git2 = "0.19"` (same version as Section 1; if Section 1 already
  added it to `pata/package/Cargo.toml`, this is a separate `pata-cli` dependency — crates don't
  share `Cargo.toml` dependency lists).
- `pata/cli/src/commands/thibitisha.rs` — add the new check function.
- `pata/cli/commands/thibitisha.md` — move "type stability" from "Not yet implemented" to
  "Checks".
- `docs/howto/04-validate-docs.md` — same update, matching the existing per-check writeup style
  already used for trait-completeness/FFI-safety in that file.

**Exact steps:**

1. Add to `pata/cli/Cargo.toml`: `git2 = "0.19"`.

2. In `pata/cli/src/commands/thibitisha.rs`, add a new function (placed after
   `enforce_ffi_signatures`, following this file's existing function-per-check pattern):
   ```rust
   /// Diff the current project's public API against the most recent `v<semver>` git tag, if one
   /// exists. No tag → no check (not an error; a project with no releases yet has nothing to be
   /// stable against). Flags: a public function present at the tag but missing now; a function
   /// present at both with a different parameter count, a different parameter type at the same
   /// position, or a different return type. Struct/trait signature changes are out of scope for
   /// this check (see docs/design/pata-production-readiness.md's Section 8 entry — extending
   /// this to structs/traits is a mechanical repeat of the same pattern once this lands, not a
   /// new design).
   fn enforce_type_stability(root: &Path) -> CliResult {
       let repo = match git2::Repository::open(root) {
           Ok(r) => r,
           Err(_) => return Ok(()), // not a git repo — nothing to diff against
       };

       let Some(tag_name) = most_recent_semver_tag(&repo) else {
           return Ok(()); // no v<semver> tags yet
       };

       let baseline_src = match read_entrypoint_at_tag(&repo, &tag_name, root) {
           Some(s) => s,
           None => return Ok(()), // tag exists but entrypoint unreadable at that rev — skip, don't fail the build over it
       };

       let baseline_tokens = match asili_lexer::tokenize(&baseline_src) {
           Ok(t) => t,
           Err(_) => return Ok(()),
       };
       let Ok(baseline_module) = asili_parser::parse_tokens(&baseline_tokens) else {
           return Ok(());
       };

       let current = compile_project(root, None)?;

       for old_fn in baseline_module.functions.iter().filter(|f| f.is_public) {
           let Some(new_fn) = current.module.functions.iter().find(|f| f.name == old_fn.name) else {
               return Err(CliError::new(
                   format!(
                       "kazi ya umma '{}' iliyopo kwenye {} imeondolewa — mabadiliko yanayovunja API",
                       old_fn.name, tag_name
                   ),
                   1,
               ));
           };
           if new_fn.params.len() != old_fn.params.len() {
               return Err(CliError::new(
                   format!(
                       "kazi ya umma '{}' idadi ya hoja imebadilika tangu {} ({} -> {})",
                       old_fn.name, tag_name, old_fn.params.len(), new_fn.params.len()
                   ),
                   1,
               ));
           }
           for (op, np) in old_fn.params.iter().zip(new_fn.params.iter()) {
               if op.ty.name != np.ty.name {
                   return Err(CliError::new(
                       format!(
                           "kazi ya umma '{}' hoja '{}' aina imebadilika tangu {} ({} -> {})",
                           old_fn.name, np.name, tag_name, op.ty.name, np.ty.name
                       ),
                       1,
                   ));
               }
           }
           if old_fn.return_type.name != new_fn.return_type.name {
               return Err(CliError::new(
                   format!(
                       "kazi ya umma '{}' aina ya kurejesha imebadilika tangu {} ({} -> {})",
                       old_fn.name, tag_name, old_fn.return_type.name, new_fn.return_type.name
                   ),
                   1,
               ));
           }
       }
       Ok(())
   }

   /// Highest `v<semver>` tag by semver ordering (not just lexical/chronological — `v2.0.0` must
   /// beat `v10.0.0`... no, must NOT beat it; standard semver Ord), or None if no tag matches.
   fn most_recent_semver_tag(repo: &git2::Repository) -> Option<String> {
       let tags = repo.tag_names(Some("v*")).ok()?;
       tags.iter()
           .flatten()
           .filter_map(|t| {
               let ver_str = t.strip_prefix('v')?;
               semver::Version::parse(ver_str).ok().map(|v| (v, t.to_string()))
           })
           .max_by(|a, b| a.0.cmp(&b.0))
           .map(|(_, tag)| tag)
   }

   /// Read the project's entrypoint source (`pata.toml`'s `[chanzo] kuingia` path, resolved the
   /// same way `load_project_config` does — but that reads the WORKING TREE's pata.toml, and the
   /// entrypoint path itself could theoretically have changed since the tag; this reads pata.toml
   /// AT the tag too, for correctness) at the given tag's git tree.
   fn read_entrypoint_at_tag(repo: &git2::Repository, tag: &str, root: &Path) -> Option<String> {
       let obj = repo.revparse_single(tag).ok()?;
       let commit = obj.peel_to_commit().ok()?;
       let tree = commit.tree().ok()?;

       let pata_toml_entry = tree.get_path(Path::new("pata.toml")).ok()?;
       let pata_toml_blob = pata_toml_entry.to_object(repo).ok()?.peel_to_blob().ok()?;
       let pata_toml_content = std::str::from_utf8(pata_toml_blob.content()).ok()?;

       // Minimal inline extraction of [chanzo] kuingia — do not call the real
       // load_project_config here, since that function reads from disk via std::fs, not from a
       // git tree object; re-implementing its 5-line [chanzo] section scan inline is simpler
       // than refactoring it to take a string.
       let mut in_chanzo = false;
       let mut entry_rel = None;
       for line in pata_toml_content.lines() {
           let line = line.trim();
           if line == "[chanzo]" {
               in_chanzo = true;
               continue;
           }
           if line.starts_with('[') {
               in_chanzo = false;
           }
           if in_chanzo {
               if let Some((k, v)) = line.split_once('=') {
                   if k.trim() == "kuingia" {
                       entry_rel = Some(v.trim().trim_matches('"').to_string());
                   }
               }
           }
       }
       let entry_rel = entry_rel.unwrap_or_else(|| "src/kuu.as".to_string());

       let entry_entry = tree.get_path(Path::new(&entry_rel)).ok()?;
       let entry_blob = entry_entry.to_object(repo).ok()?.peel_to_blob().ok()?;
       std::str::from_utf8(entry_blob.content()).ok().map(|s| s.to_string())

       // NOTE: matches root only to satisfy the function signature parity with other CliError
       // sites in this file — root itself isn't used for git tree reads (the tree IS the
       // historical filesystem state), only for consistency with this file's existing functions
       // that all take `root: &Path`. Silence an unused-variable warning by prefixing the param
       // with `_root` if this note is inaccurate after implementation — check with `cargo build`.
   }
   ```
   Add `semver = "1"` to `pata/cli/Cargo.toml` too (separate crate from `pata-package`, needs its
   own copy of the dependency).

3. Wire it into `run()`: add `enforce_type_stability(Path::new("."))?;` right after the existing
   `enforce_ffi_signatures(&output.module)?;` call.

4. Update the top-of-file comment block (the one currently listing what's checked vs. not) to move
   type-stability from "Not implemented" to the checked list.

**Acceptance check:**
```bash
cargo build -p pata-cli 2>&1 | tail -30
```
Then a real end-to-end test (this cannot be a unit test alone, since it needs actual git tag
history — write this as an integration test in `pata/cli/src/commands/thibitisha.rs`'s test
module, using `std::process::Command::new("git")` to init a repo, commit, tag `v1.0.0`, commit a
breaking change, then run `thibitisha` and assert failure):
```rust
#[test]
fn fails_when_public_function_signature_changed_since_last_tag() {
    let _guard = TEST_CWD_LOCK.lock().expect("lock");
    // build a temp project (reuse temp_project_no_public's shape but with one public fn),
    // `git init`, `git add -A`, `git commit`, `git tag v1.0.0`,
    // then edit the public function's parameter type on disk (no new commit needed —
    // enforce_type_stability diffs the WORKING TREE's compiled module against the TAG,
    // not two commits),
    // run(&[]), assert it errs with a message containing "imebadilika tangu v1.0.0"
}
```
Write this test fully (the comment above is the exact sequence; implement it using
`std::process::Command` calls to the `git` binary directly for repo setup — that's simpler and
more obviously correct than driving `git2` for the test's own setup phase, even though the
production code uses `git2`). Confirm `git --version` succeeds in this environment first
(`git --version` — if it fails, this test cannot run in this environment; note that in the PR
description rather than silently skipping it).

---

## Section 9: `pata jaribu` parallel execution

**Scope boundary, decided:** parallel execution only. Timeouts, fixtures, and coverage
instrumentation remain deferred (see bottom) — each is independently large and this section is
already substantial on its own.

**Files to touch:**
- `pata/cli/src/pipeline/compile.rs` — find `execute_tests` (referenced by `run_project_tests`;
  `Read` the full function first — it wasn't fully read in this conversation's earlier
  exploration, only confirmed to exist and take `fail_fast: bool`).
- `pata/cli/src/commands/jaribu.rs` — add `--nyuzi <n>` flag.
- `pata/cli/Cargo.toml` — add `rayon = "1"` (simplest correct parallel-iterator crate; avoids
  hand-rolling a thread pool).

**Exact steps:**

1. `Read` `execute_tests`'s current implementation in full before changing anything — this spec
   cannot give exact replacement code without seeing the real function body (unlike every other
   section above, where the target function was already read in full during this conversation).
   Confirm its signature and exactly how it currently iterates `modules_and_tests` and runs each
   test (almost certainly a `for` loop calling into `core/evaluator`'s `run_main`-equivalent
   per-test, given `fail_fast: bool` implies early-exit logic already exists that a parallel
   version must preserve or explicitly document as incompatible with `--simama-haraka`).

2. Add `rayon = "1"` to `pata/cli/Cargo.toml`.

3. Add a `--nyuzi <n>` flag to `jaribu.rs`'s `parse_args` (Swahili: "nyuzi" = threads), defaulting
   to `1` (sequential, today's exact behavior) when not passed — **this default is required**, not
   optional: changing the default behavior of an existing command without an explicit flag is a
   breaking change to `pata jaribu`'s existing contract (`jaribu.md` says nothing about
   parallelism today) and must not happen silently.

4. When `--nyuzi` is `> 1`: **`--simama-haraka`/`fail_fast` becomes best-effort, not exact** — the
   first-discovered failure across all parallel workers stops new test starts, but tests already
   in flight on other threads complete. Document this precisely in `jaribu.md`'s
   `--simama-haraka` entry (add a sentence: "with `--nyuzi > 1`, stops launching new tests after
   the first failure but does not interrupt tests already running") rather than silently changing
   what `--simama-haraka` means.

5. Implementation shape (exact approach, not exact code, since step 1's `Read` determines the
   precise integration point): wrap the per-test execution loop's body in
   `modules_and_tests.par_iter()` (from `rayon::prelude::*`) instead of `.iter()` when `nyuzi > 1`,
   collecting `TestResult`s into a `Vec` via `.collect()` same as today, then sort the collected
   results by original input order before printing (rayon's `par_iter` does not guarantee output
   order — sorting restores the deterministic print order `pata jaribu`'s existing tests likely
   assert on; check `jaribu.rs`'s and `compile.rs`'s existing tests for any order-dependent
   assertions on printed output before finalizing this).
   For `nyuzi == 1`, keep the exact existing sequential code path completely unchanged (an `if
   nyuzi > 1 { /* rayon path */ } else { /* existing loop, byte-for-byte unmodified */ }` branch) —
   this guarantees zero behavior change for every existing caller/test that doesn't pass the new
   flag.

**Acceptance check:**
```bash
cargo build -p pata-cli 2>&1 | tail -30
cargo test -p pata-cli 2>&1 | tail -40
```
Every existing test must still pass unmodified (proves the `nyuzi == 1` path is truly untouched).
Then add one new test: a temp project with 3+ `#[jaribio]` functions, run with `--nyuzi 4`, assert
all pass and the summary line (`"jumla: 3 | sawa: 3 | kosa: 0"`) is correct regardless of
completion order.

---

## Section 10: `pata jaribu` per-test timeout

**Decision made:** kill-mechanism is thread-based, not signal/interrupt-based. `Module`/`Function`
(`core/parser/src/ast.rs`) are plain owned data with no `Rc`/`RefCell` — confirmed by inspection,
auto-derived `Send` applies — so a runaway test's evaluation can run on a spawned
`std::thread::spawn`, joined with a timeout via a chalfneled result. A thread that times out is
**abandoned, not killed** (Rust has no safe thread-kill primitive) — its evaluation keeps running
in the background until it naturally finishes or the process exits, but `pata jaribu` stops
waiting on it and reports it as a timeout failure. This is the same tradeoff `cargo test`'s own
`--test-threads` timeout wrappers accept; documented explicitly in `jaribu.md` so it's not a
surprise.

**Files to touch:**
- `pata/cli/src/pipeline/compile.rs` — wrap the per-test call in `execute_tests` (the function
  identified/read in full during Section 9).
- `pata/cli/src/commands/jaribu.rs` — add `--muda-kikomo <sekunde>` flag.
- `pata/cli/commands/jaribu.md` — document it.

**Exact steps:**

1. Add a helper near `execute_tests` in `compile.rs`:
   ```rust
   use std::sync::mpsc;
   use std::time::Duration;

   /// Run `run_test_with_module` on a spawned thread, joined with a timeout. On timeout, the
   /// spawned thread is abandoned (not killed — Rust has no safe thread-kill primitive) and this
   /// returns a synthetic failing `TestResult`. The abandoned thread's evaluation keeps running
   /// until it finishes naturally or the process exits; it cannot corrupt `pata jaribu`'s own
   /// state since `module`/`function` are cloned into the thread, not shared.
   fn run_test_with_timeout(
       module: Module,
       function: Function,
       timeout: Duration,
   ) -> asili_evaluator::TestResult {
       let (tx, rx) = mpsc::channel();
       let name_for_timeout = function.name.clone();
       std::thread::spawn(move || {
           let result = asili_evaluator::run_test_with_module(&module, &function);
           let _ = tx.send(result);
       });
       match rx.recv_timeout(timeout) {
           Ok(result) => result,
           Err(_) => asili_evaluator::TestResult {
               name: name_for_timeout,
               passed: false,
               message: format!("muda umekwisha (>{:?}) — jaribio limeachwa likiendelea nyuma", timeout),
           },
       }
   }
   ```
   `Module`/`Function` must be moved (owned), not borrowed, into the closure — this means
   `execute_tests`'s existing loop (which per Section 9's investigation iterates
   `&modules_and_tests: &[(Module, Function)]`) must `.clone()` each pair before calling this
   function instead of passing references. Confirm `Module`/`Function` both derive `Clone`
   (`Module` does not show a `Clone` derive in the struct definition read during this session —
   **check this first**: `grep -n "derive.*Clone" core/parser/src/ast.rs | head -3` before writing
   this section's code; if `Module` doesn't derive `Clone`, add `Clone` to its derive list as a
   prerequisite step, and re-run the full workspace test suite after that alone, since widening a
   shared AST type's derives can have unexpected knock-on effects e.g. on `#[derive(Debug)]`
   bounds elsewhere — verify with `cargo build --workspace` before proceeding to the rest of this
   section).

2. In `execute_tests`, add a `timeout: Option<Duration>` parameter (threaded through from
   `run_project_tests`'s own new `timeout` parameter, itself threaded from `jaribu.rs`'s new flag).
   When `Some(t)`, call `run_test_with_timeout(module.clone(), function.clone(), t)` instead of the
   existing direct call; when `None` (default — no flag passed), keep today's exact direct-call
   behavior unchanged (same "don't change default behavior silently" rule as Section 9's `--nyuzi`
   flag).

3. In `jaribu.rs`, add `--muda-kikomo <sekunde>` (Swahili: "muda kikomo" = time limit), parsed as
   an `f64` (seconds, allowing `0.5` for sub-second limits), converted via
   `Duration::from_secs_f64`. Validate `> 0.0`, matching `thibitisha.rs`'s `--kiwango-cha-jaribio`
   validation style exactly (same error-message shape: `"--muda-kikomo inahitaji thamani chanya
   (sekunde)"` on invalid input).

4. Update `jaribu.md`: add `--muda-kikomo <sekunde>` to Inputs, and a new Failures bullet: "a test
   exceeding `--muda-kikomo` is reported as failed; the underlying thread is not forcibly
   terminated (documented Rust limitation) and may continue running in the background."

**Acceptance check:**
```bash
cargo build -p pata-cli 2>&1 | tail -30
cargo test -p pata-cli 2>&1 | tail -40
```
All existing tests pass unmodified. Add one new test: a temp project with a test function
containing an infinite loop (`kama kweli { }` — confirm this is valid infinite-loop syntax by
checking `docs/language/`'s control-flow doc first, or use `weka x = 0\nnjia_rudia { }`-equivalent
per this language's actual loop syntax — grep `docs/language/04-udhibiti-wa-mtiririko.md` or
equivalent for the exact infinite-loop construct before writing this test, since guessing syntax
here would produce a test that fails to compile rather than one that hangs) run with
`--muda-kikomo 1`, asserting the command returns within a few seconds (not hangs) and reports that
test as failed with a message containing `"muda umekwisha"`.

---

## Section 11: `pata jaribu` setup/teardown fixtures

**Decision made:** two new attributes, `#[kabla]` (before each test in the same file) and
`#[baada]` (after each test in the same file) — matching this language's existing
`#[jaribio]`-style bare attribute convention (`core/parser/src/semantic/analyzer.rs`'s
`allowed_attrs` list already includes `"jaribio"`, `"sharti"`, `"ndani"`, `"kiunganishi"` — this
adds two more to that same list). Scope is **per-file**, not per-project or per-function-group —
the simplest rule that's still useful, matching how `pata jaribu` already discovers tests
per-file (`collect_as_files(&src_dir)` in `run_project_tests`). No fixture *value* passing (a
`#[kabla]` function can't hand a resource to the test it precedes) — fixtures are pure
side-effecting hooks (e.g. reset a file, clear a counter global). Passing values would need a
first-class dependency-injection design this spec deliberately avoids as out of scope creep.

**Files to touch:**
- `core/parser/src/semantic/analyzer.rs` — add `"kabla"`, `"baada"` to `allowed_attrs` (the exact
  list confirmed at line 235 in earlier exploration this conversation:
  `["jaribio", "sharti", "ndani", "kiunganishi"]`).
- `core/parser/src/ast.rs` — no changes needed; `Function.attrs: Vec<Attribute>` and
  `Attribute.name: String` already generically support any attribute name; `is_test` is a
  parser-computed bool (check `core/parser/src/parse.rs` for exactly where `is_test` gets set from
  `attrs` — likely `attrs.iter().any(|a| a.name == "jaribio")` — mirror this pattern for two new
  computed bools if adding them to `Function` is cleaner than re-scanning `attrs` at use time; a
  reader who checks `f.attrs.iter().any(|a| a.name == "kabla")` at use time needs no `Function`
  struct change at all, which is simpler and is the recommended approach — do not add new
  `Function` fields for this).
- `pata/cli/src/pipeline/compile.rs` — modify `run_project_tests`'s per-file loop.

**Exact steps:**

1. Add `"kabla"` and `"baada"` to the `allowed_attrs` array in
   `core/parser/src/semantic/analyzer.rs` (the exact line confirmed earlier this conversation:
   `let allowed_attrs = ["jaribio", "sharti", "ndani", "kiunganishi"];` becomes
   `let allowed_attrs = ["jaribio", "sharti", "ndani", "kiunganishi", "kabla", "baada"];`).

2. In `run_project_tests` (`pata/cli/src/pipeline/compile.rs`), the per-file loop currently builds
   `modules_and_tests: Vec<(Module, Function)>` by finding every `#[jaribio]` function per parsed
   file (confirm the exact collection logic by reading the function in full — Section 9 already
   required this read). Add, per file, **before** collecting the test list: find the first
   function (if any) whose `attrs` contains `{ name: "kabla", .. }`, and the first whose `attrs`
   contains `{ name: "baada", .. }` (first-match only — if a file has two `#[kabla]` functions,
   only the first runs; do not silently run both, and do not error either, matching this codebase's
   generally-permissive style elsewhere — this is a deliberate simplicity choice, not an oversight,
   document it as such in a code comment at the point of implementation).

3. Change the per-test execution to, for each test in a file that has a `#[kabla]`/`#[baada]`
   function: call `run_function(module, kabla_fn.name, vec![])` (ignoring its return value, but
   propagating a panic — if setup panics, the test it precedes is reported as failed with the
   setup's panic message, prefixed `"kabla imeshindwa: "`) immediately before running the test
   itself, and `run_function(module, baada_fn.name, vec![])` immediately after — **always run
   `#[baada]` even if the test itself failed**, mirroring how most test frameworks guarantee
   teardown runs (wrap the test-execution + baada call in a pattern that runs baada regardless of
   the test's pass/fail, e.g. compute the test result first, then unconditionally invoke baada
   before returning that result — do not use Rust's own `Drop`/panic-unwind for this, since
   `run_function`'s existing error handling already returns `Result` rather than panicking through
   normal control flow, per `core/evaluator/src/lib.rs`'s `run_test_with_module`'s existing
   `match run_function(...) { Ok(_) => ..., Err(EvalError::Panic(msg)) => ..., Err(e) => ... }`
   pattern — follow that exact pattern, don't introduce unwinding where none exists today).

4. `#[kabla]`/`#[baada]`-tagged functions themselves must be excluded from the normal test list
   (they are not tests, even though they live in the same file) — when building
   `modules_and_tests`, skip any function whose `attrs` contains `"kabla"` or `"baada"` even if it
   also happens to be tagged `#[jaribio]` (document: tagging a function both `#[jaribio]` and
   `#[kabla]` is nonsensical and this makes it a no-op fixture rather than erroring, matching this
   codebase's generally-permissive-over-strict style for edge cases like this).

**Acceptance check:**
```bash
cargo build --workspace 2>&1 | tail -30
cargo test -p pata-cli 2>&1 | tail -40
```
All existing tests pass. Add a new integration test: a temp project with a module-level mutable
global-equivalent (check this language's actual mechanism for a test to observe fixture
side-effects across function calls — likely a `thabiti`/file it writes to, since there's no shared
mutable module state without `Kasha_GC`/similar; simplest working approach: `#[kabla]` writes a
marker to a temp file via `leta faili`, the test asserts the file's content, `#[baada]` deletes the
file, and a second test in the same file asserts the file does NOT exist at its start, proving
`#[baada]` ran after the first test before the second one started) — run `pata jaribu`, assert both
tests pass.

---

## Section 12: `pata jaribu` code coverage (line-level, evaluator-instrumented)

**Decision made:** line coverage, not branch coverage (branch coverage needs control-flow-graph
construction this spec is not attempting) — instrumented at the evaluator level via a callback
hook, not an external tool (no existing Rust coverage tool understands `.as` source, so `tarpaulin`
-style external instrumentation is not applicable; this must be native to `core/evaluator`).

**Files to touch:**
- `core/evaluator/src/lib.rs` — add an optional coverage-recording hook to the eval path.
- `core/evaluator/src/eval/` (whichever file contains the statement-execution dispatch loop — `grep
  -rn "fn eval_stmt\|fn exec_stmt" core/evaluator/src/eval/*.rs` to find it exactly) — record each
  executed statement's line number.
- `pata/cli/src/pipeline/compile.rs` — thread a coverage collector through `run_project_tests`.
- `pata/cli/src/commands/jaribu.rs` — add `--chanjo` (coverage) flag.

**Exact steps:**

1. In `core/evaluator/src/lib.rs`, add a new type:
   ```rust
   use std::sync::{Arc, Mutex};
   use std::collections::HashSet;

   /// Line numbers actually executed during a run, shared across however many statement
   /// evaluations occur. `Arc<Mutex<..>>` because the timeout mechanism (Section 10) may run
   /// evaluation on a spawned thread — this must be safely shareable across that boundary too,
   /// not just usable single-threaded.
   #[derive(Clone, Default)]
   pub struct CoverageRecorder(Arc<Mutex<HashSet<usize>>>);

   impl CoverageRecorder {
       pub fn new() -> Self {
           Self::default()
       }
       pub fn record(&self, line: usize) {
           if let Ok(mut set) = self.0.lock() {
               set.insert(line);
           }
       }
       pub fn lines_hit(&self) -> HashSet<usize> {
           self.0.lock().map(|s| s.clone()).unwrap_or_default()
       }
   }
   ```

2. Find the statement-dispatch function (the `grep` above) and confirm its exact signature before
   editing. It almost certainly takes `&Stmt` (or similar) and an environment/runtime reference.
   Add a `coverage: Option<&CoverageRecorder>` parameter threaded through every recursive call in
   that function (this touches every call site of the statement evaluator — a mechanical but
   wide-reaching change; use `grep -rn "eval_stmt(\|exec_stmt("` — substitute the real function
   name found above — across `core/evaluator/src/` to enumerate every call site that needs the new
   parameter added). At the top of the function body, before dispatching on the statement variant,
   add:
   ```rust
   if let Some(cov) = coverage {
       cov.record(stmt_line_number); // substitute the real field access for the current Stmt's line
   }
   ```
   (Every `Stmt` variant in `core/parser/src/ast.rs` carries a `line: usize` field — confirmed
   directly in this conversation's exploration of `Stmt::Let`/`Stmt::Assign` above; extracting it
   generically needs a `match` arm per variant or a shared accessor method — check whether `Stmt`
   already has a `.line()` method via `grep -n "impl Stmt" core/parser/src/ast.rs`; if not, add one
   as a small prerequisite: a `match self { Stmt::Let { line, .. } => *line, ... }` covering every
   variant.)

3. Add a matching public entry point in `core/evaluator/src/lib.rs`:
   ```rust
   /// Same as `run_test_with_module`, but records which source lines were executed into
   /// `coverage`. Pass a fresh `CoverageRecorder` per test if you want per-test coverage, or share
   /// one recorder across a whole run's tests for project-wide coverage — the caller decides.
   pub fn run_test_with_coverage(
       module: &Module,
       function: &Function,
       coverage: &CoverageRecorder,
   ) -> TestResult {
       // same body as run_test_with_module, but threading Some(coverage) into the eval call
       // instead of run_test_with_module's implicit None
   }
   ```

4. In `pata/cli/src/pipeline/compile.rs`, add a `--chanjo`-gated path in `run_project_tests`: when
   coverage is requested, use `run_test_with_coverage` with one shared `CoverageRecorder` across
   all tests in the run, then after all tests complete, compute: total executable statement lines
   across every function in every collected module (walk `module.functions[*].body.statements`,
   counting recursively into nested blocks — `If`/`While`/`Kama`-equivalent bodies — via the same
   `Stmt::line()` accessor from step 2) versus `coverage.lines_hit().len()`, and print:
   ```
   chanjo: <hit>/<total> mistari (<percent>%)
   ```
   Exit code is unaffected by coverage (informational only in this pass — a `--chanjo-kiwango
   <pct>`-style enforced-threshold flag, mirroring `thibitisha`'s `--kiwango-cha-jaribio`, is a
   reasonable follow-up but not required for this section's acceptance check).

**Acceptance check:**
```bash
cargo build --workspace 2>&1 | tail -40
cargo test --workspace 2>&1 | grep -E "FAILED|error:"
```
Must be empty (this section touches the evaluator's core statement-dispatch signature — a
wide-reaching, easy-to-break change; full-workspace green is mandatory, not optional, same as
Section 7). Then:
```bash
cd examples/sifa && cargo run -q -p pata-cli -- jaribu --chanjo 2>&1 | tail -5
```
Must print a `chanjo: N/M mistari (X%)` line with `N <= M` and `X` between 0 and 100.

---

## Section 13: `pata-lint` — new rule: unused local variables

**Decision made:** starting with exactly one new rule (not all four originally listed — unused
vars, shadowing, unreachable code, redundant match arms) to keep this section's acceptance check
tight and mechanical. The other three follow the identical pattern established here — see "Repeat
pattern for the remaining three rules" at the end of this section instead of a full separate
writeup for each (writing all four in full would just be this same section's shape copy-pasted
three more times with a different detection predicate each).

**New code:** `LINT004` — a `weka` (let) binding never read again in its enclosing block, and never
reassigned via `Stmt::Assign` either (a variable that's only ever written, never read, is dead).

**Files to touch:**
- `pata/lint/src/rules/naming.rs` — despite the name, this is where `LINT001-003` (naming rules)
  live; `LINT004` is not a naming rule. Create a **new file**: `pata/lint/src/rules/unused.rs`.
- `pata/lint/src/rules/mod.rs` — add `pub mod unused;`.
- `pata/lint/src/lib.rs` — add `lints.extend(rules::unused::check_unused_bindings(&module));` to
  `lint_source`.

**Exact steps:**

1. Create `pata/lint/src/rules/unused.rs`:
   ```rust
   //! Unused-binding lint rules

   use asili_diagnostics::Diagnostic;
   use asili_parser::{Block, Expr, Function, Module, Stmt};
   use std::collections::HashSet;

   /// LINT004: a `weka` binding that's never read again within its enclosing function body (only
   /// ever written — declared, and optionally reassigned, but never used in an expression). Scope
   /// is function-local and non-nested-aware in this first pass: a variable shadowed in an inner
   /// block is treated as a separate name for this check's purposes, matching how the language's
   /// own scoping actually works (an inner `weka x` shadows, it doesn't reuse, the outer `x`'s
   /// slot) — see docs/language's scoping doc if this assumption needs re-checking before
   /// implementation.
   pub fn check_unused_bindings(module: &Module) -> Vec<Diagnostic> {
       let mut diags = Vec::new();
       for func in &module.functions {
           check_block(&func.body, &mut diags);
       }
       diags
   }

   fn check_block(block: &Block, diags: &mut Vec<Diagnostic>) {
       let mut declared: Vec<(String, usize)> = Vec::new(); // (name, line) in declaration order
       let mut read: HashSet<String> = HashSet::new();

       for stmt in &block.statements {
           collect_reads_in_stmt(stmt, &mut read);
           if let Stmt::Let { name, line, .. } = stmt {
               declared.push((name.clone(), *line));
           }
       }

       for (name, line) in &declared {
           if !read.contains(name) {
               diags.push(
                   Diagnostic::new(
                       "LINT004",
                       format!("kigezo '{}' hakitumiki baada ya kutangazwa", name),
                   )
                   .with_stage("ukaguzi")
                   .with_span(*line, 1),
               );
           }
       }

       // Recurse into nested blocks (If/While/etc. bodies) — exact recursion shape depends on
       // Stmt's real variant list; check core/parser/src/ast.rs's full Stmt enum (only Let/Assign/
       // If were confirmed in this conversation's exploration) and add a match arm per
       // block-containing variant (If's then/else blocks, While's body, etc.), calling
       // check_block on each nested Block found.
   }

   /// Walks `stmt`'s expressions (and nested statements) collecting every identifier read — NOT
   /// including the LHS of a `Stmt::Let`/`Stmt::Assign` itself (that's a write, not a read; the
   /// RHS `value` expression of both IS a read of whatever it references). Exact recursion
   /// depends on Expr's full variant list — check core/parser/src/ast.rs's Expr enum in full
   /// before implementing (not read during this spec's own exploration — this is the one
   /// remaining unknown in this section, confirm it first with `grep -n "pub enum Expr" -A 40
   /// core/parser/src/ast.rs`), and add a match arm per variant that can reference an identifier
   /// (Expr::Ident(name) at minimum; also function-call arguments, binary-op operands, method-call
   /// receivers, struct-literal field values, etc. — anywhere a variable name can legally appear
   /// as a read).
   fn collect_reads_in_stmt(stmt: &Stmt, read: &mut HashSet<String>) {
       match stmt {
           Stmt::Let { value, .. } => collect_reads_in_expr(value, read),
           Stmt::Assign { value, .. } => collect_reads_in_expr(value, read),
           // TODO: add every other Stmt variant, recursing into contained expressions and blocks.
           _ => {}
       }
   }

   fn collect_reads_in_expr(expr: &Expr, read: &mut HashSet<String>) {
       // TODO: match every Expr variant; at minimum Expr::Ident(name) => { read.insert(name.clone()); }
       let _ = (expr, read);
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use asili_lexer::tokenize;
       use asili_parser::parse_tokens;

       fn parse(src: &str) -> Module {
           let tokens = tokenize(src).expect("tokenize");
           parse_tokens(&tokens).expect("parse")
       }

       #[test]
       fn flags_unused_binding() {
           let module = parse("kazi f() -> Tupu {\n  weka x = 1\n  rejesha Tupu\n}\n");
           let diags = check_unused_bindings(&module);
           assert!(diags.iter().any(|d| d.code == "LINT004"), "{:?}", diags);
       }

       #[test]
       fn does_not_flag_used_binding() {
           let module = parse("kazi f() -> Tupu {\n  weka x = 1\n  chapisha(x)\n  rejesha Tupu\n}\n");
           let diags = check_unused_bindings(&module);
           assert!(!diags.iter().any(|d| d.code == "LINT004"), "{:?}", diags);
       }
   }
   ```

2. **Before this compiles**, `collect_reads_in_stmt`/`collect_reads_in_expr`'s `TODO`s must be
   filled in against the real `Stmt`/`Expr` enums — this is the one place in this entire spec where
   the exact enum shape wasn't already confirmed during this conversation's exploration (unlike
   every other section). Run `grep -n "pub enum Expr" -A 60 core/parser/src/ast.rs` and `grep -n
   "pub enum Stmt" -A 80 core/parser/src/ast.rs` first, then write a match arm per variant. This is
   mechanical (one arm per variant, recursing into whatever sub-expressions/sub-blocks that variant
   contains) but must be done against the real enum, not guessed.

3. Add `pub mod unused;` to `pata/lint/src/rules/mod.rs`, and in `pata/lint/src/lib.rs`'s
   `lint_source`, add: `lints.extend(rules::unused::check_unused_bindings(&module));`.

**Repeat pattern for the remaining three rules** (each is the same shape as above — a new file
under `pata/lint/src/rules/`, registered the same two ways, with its own detection predicate over
the same `Module`/`Function`/`Block`/`Stmt` types):
- **LINT005 (shadowing)**: within one `Block`, a `weka` declaring a name already declared earlier
  in the *same* block (not an outer/nested one — same-block only, the least controversial
  definition of shadowing to flag). Detection: track declared names in declaration order per block
  (same `declared` vec shape as `check_block` above); if a name appears twice, flag the second
  occurrence.
- **LINT006 (unreachable code after `rejesha`)**: within one `Block`, any statement appearing after
  a `Stmt::Return`-equivalent (confirm the exact variant name — likely `Stmt::Return` or
  `Stmt::Rejesha`, check the real enum) at the same block level is unreachable. Detection: iterate
  a block's statements; once a return-variant is seen, flag every subsequent statement in that same
  vec.
- **LINT007 (redundant `linganisha` arms)**: a `linganisha` (match) with two arms whose patterns
  are structurally identical (same `Pattern` value via `PartialEq`, if `Pattern` derives it — check
  `core/parser/src/ast.rs`) makes the second arm dead. Detection: within one `Stmt::Match`-
  equivalent, collect all arm patterns, flag any pattern equal to an earlier one in the same match.

Each of these three gets its own 2-test acceptance pair (trigger + no-trigger), same style as
Section 4's existing-rule tests.

**Acceptance check (for LINT004 alone, as the fully-specified one above; repeat for LINT005-007
once written following the pattern):**
```bash
cargo build -p pata-lint 2>&1 | tail -30
cargo test -p pata-lint 2>&1 | tail -20
```

---

## Section 14: `pata-lint` — per-rule configurability

**Decision made:** severity/threshold overrides via a new `[lint]` table in `pata.toml` (Swahili
key names, matching every other `pata.toml` section's convention — `[jumla]`, `[tegemezi]`,
`[jenga]`). No inline suppression syntax (`#[ruhusu(LINT101)]`) in this pass — that needs parser
support for a new attribute-argument shape (`#[jaribio]`-style bare attributes don't carry a
lint-code argument today), which is a larger change than a manifest-level toggle. Only two knobs:
disable a rule entirely, and override `LINT101`'s hardcoded line-count threshold (the only rule
with a numeric parameter today).

**Files to touch:**
- `pata/cli/src/pipeline/project.rs` — extend `ProjectConfig`/`load_project_config` to read a new
  `[lint]` section.
- `pata/lint/src/lib.rs` — change `lint_source`'s signature to accept config.
- Every call site of `lint_source` (`grep -rn "lint_source(" pata/ --include="*.rs"` to enumerate
  them exactly — likely `pata/lint/src/main.rs` and `pata/lsp/src/diagnostics.rs`) — update to pass
  config (or `LintConfig::default()` where no `pata.toml` is available, e.g. the LSP's per-file
  linting which may run before a project root is known).

**Exact steps:**

1. In `pata/lint/src/lib.rs`, add:
   ```rust
   #[derive(Clone, Debug, Default)]
   pub struct LintConfig {
       /// Lint codes to skip entirely (e.g. "LINT101").
       pub disabled: std::collections::HashSet<String>,
       /// Override for LINT101's line-count threshold; None = use the hardcoded default (confirm
       /// the exact current default by re-reading pata/lint/src/rules/style.rs before writing this
       /// — it was 50 as of this conversation's earlier exploration, but re-verify, do not assume
       /// it hasn't changed).
       pub max_function_lines: Option<usize>,
   }
   ```
   Change `lint_source`'s signature to `pub fn lint_source(source: &str, config: &LintConfig) ->
   Result<Vec<Diagnostic>, String>`, and at the end (after collecting `lints`), filter:
   `lints.retain(|d| !config.disabled.contains(&d.code));`. Thread `config.max_function_lines`
   into `rules::style::check_style_issues` by changing that function's signature to accept an
   `Option<usize>` threshold parameter too (defaulting to the current hardcoded value when `None`).

2. In `pata/cli/src/pipeline/project.rs`, extend `ProjectConfig` with a new field:
   ```rust
   pub lint: pata_lint::LintConfig, // requires pata-lint as a pata-cli dependency if not already present — check pata/cli/Cargo.toml first
   ```
   and in `load_project_config`'s line-scanning loop, add a new `"lint"` section case parsing:
   ```toml
   [lint]
   zima = ["LINT101", "LINT203"]
   mistari_ya_juu = 80
   ```
   (`zima` = "disable", a comma-separated-in-brackets list — check how `pata.toml`'s hand-rolled
   parser already handles any existing array-valued key, if one exists, and mirror that exact
   parsing approach; if none exists yet, split on `,` after trimming `[`/`]`/quotes, the simplest
   viable approach consistent with this parser's existing permissive style) into
   `cfg.lint.disabled`, and `mistari_ya_juu` (a plain integer) into `cfg.lint.max_function_lines`.

3. Update every `lint_source` call site found in step "Files to touch" above to pass
   `&cfg.lint` where a `ProjectConfig` is available, or `&LintConfig::default()` where it isn't.

**Acceptance check:**
```bash
cargo build --workspace 2>&1 | tail -40
cargo test --workspace 2>&1 | grep -E "FAILED|error:"
```
Must be empty — this section changes a public function signature (`lint_source`) that multiple
crates call, so full-workspace green is mandatory. Add one new test in `pata/lint`'s own test
module: lint a source string containing a 60-line function with `LintConfig { disabled:
["LINT101"].into(), .. }`, assert no `LINT101` diagnostic appears despite the function being long
enough to normally trigger it.

---

## Section 15: shared `pata-core` crate — resolver extraction only

**Decision made:** narrower than the original proposal. Extract only the module resolver
(`resolve.rs` + `interface_registry.rs`) into a new library crate `pata/core`, leaving the
formatter where it already landed (`pata/fmt`, done by a prior session) and leaving `pata-cli`'s
other pipeline modules (`compile.rs`, `project.rs`, `sharti.rs`, `format.rs`) where they are —
those aren't needed by `pata-lsp`, so moving them would be motion without payoff. This directly
retires `pata/lsp/src/workspace.rs`'s parallel reimplementation (its own doc comment, lines 21-26,
names this exact extraction as the alternative it rejected only because it was "a larger structural
change than this fix needs" at the time — that constraint no longer holds now that Sections 1-14
have already exercised and hardened the resolver).

**Files to touch:**
- **New crate directory** `pata/core/` with its own `Cargo.toml` (name `pata-core`, `lib.rs`
  re-exporting what moves).
- `pata/core/src/resolve.rs` — moved from `pata/cli/src/pipeline/resolve.rs` (`git mv`, not
  copy-then-delete, to preserve file history).
- `pata/core/src/interface_registry.rs` — moved from `pata/cli/src/pipeline/interface_registry.rs`.
- `pata/cli/src/pipeline/resolve.rs`, `pata/cli/src/pipeline/interface_registry.rs` — deleted;
  `pata/cli/Cargo.toml` gains `pata-core = { path = "../core" }`; every `pata/cli/src/` file that
  did `use crate::pipeline::resolve::...` or `use crate::pipeline::interface_registry::...` changes
  to `use pata_core::...` (`grep -rln "pipeline::resolve\|pipeline::interface_registry"
  pata/cli/src/` to enumerate every file needing this import-path update).
- `pata/lsp/Cargo.toml` — gains `pata-core = { path = "../core" }`.
- `pata/lsp/src/workspace.rs` — **deleted entirely**, replaced by direct calls into
  `pata_core::resolve`/`pata_core::interface_registry` from wherever `workspace.rs`'s
  `WorkspaceIndex` (or equivalent — re-read the file's real public API in full before deleting it,
  since only its module-level doc comment was read during this conversation's exploration, not its
  full implementation) was consumed by the rest of `pata-lsp`.

**Exact steps:**

1. Create `pata/core/Cargo.toml`:
   ```toml
   [package]
   name = "pata-core"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   asili-lexer = { path = "../../core/lexer" }
   asili-parser = { path = "../../core/parser" }
   asili-diagnostics = { path = "../../core/diagnostics" }
   pata-package = { path = "../package" }
   ```
   (Match version numbers/edition to whatever `pata/cli/Cargo.toml` currently uses — `Read` it
   first, do not assume `2021`.) Add `"pata/core"` to the workspace `[workspace] members` list in
   the root `Cargo.toml`.

2. `git mv pata/cli/src/pipeline/resolve.rs pata/core/src/resolve.rs` and `git mv
   pata/cli/src/pipeline/interface_registry.rs pata/core/src/interface_registry.rs`. Create
   `pata/core/src/lib.rs`:
   ```rust
   pub mod resolve;
   pub mod interface_registry;
   ```
   Fix internal `use` paths inside the two moved files: anything reading `use crate::pipeline::
   project::Dependency` (confirmed present in the original `resolve.rs`, per this conversation's
   earlier exploration) now needs `pata_package::manifest::Dependency` or wherever `Dependency`
   actually lives post-move — **this is the one real design fork in this section**: `Dependency`
   (the CLI's own enum, `pata/cli/src/pipeline/project.rs:6-16`, distinct from
   `pata_package::manifest::Dependency`) is defined in `pata-cli`, which `pata-core` cannot depend
   on (same cycle problem that motivated this extraction in the first place). Resolution: move
   `Dependency` (the enum) itself into `pata-core` too (it's small — a 2-variant enum plus a
   `Display` impl, per the code read earlier this conversation) as `pata_core::Dependency`, and
   have `pata/cli/src/pipeline/project.rs` re-export it (`pub use pata_core::Dependency;`) so every
   existing `pata-cli` call site that names `crate::pipeline::project::Dependency` keeps compiling
   unchanged.

3. Update `pata/cli/src/pipeline/mod.rs` (or wherever the `pub mod resolve;`/`pub mod
   interface_registry;` declarations currently live — check `pata/cli/src/pipeline/mod.rs` first)
   to remove those two lines, and fix every importing file found via the `grep` in "Files to touch"
   above.

4. In `pata/lsp/`, add the `pata-core` dependency, then rewrite whatever consumed
   `workspace.rs`'s `WorkspaceIndex` to instead call `pata_core::resolve::resolve_all` /
   `pata_core::interface_registry::InterfaceRegistry` directly — the real signatures of both are
   already fully known from this conversation's own earlier work on `interface_registry.rs`
   (Section on `.asi` trait tracking). Delete `pata/lsp/src/workspace.rs`.

**This section carries real risk — more than any other in this spec** (it's a structural move
across a crate boundary, touching test infrastructure in both `pata-cli` and `pata-lsp`). Do not
attempt it in the same sitting as any other section; run the acceptance check below in isolation,
and if `cargo test --workspace` shows *any* new failure versus the baseline captured before this
section started, stop and do not proceed to committing — revert (`git checkout -- pata/core
pata/cli pata/lsp Cargo.toml` — after confirming via `git status` that these are the only paths
touched by this section) rather than attempting to debug forward from a partially-broken
cross-crate move.

**Acceptance check:**
```bash
cargo build --workspace 2>&1 | tail -60
cargo test --workspace 2>&1 | grep -E "FAILED|error:"
grep -rn "workspace.rs" pata/lsp/src/*.rs  # must return nothing — file is gone, no dangling references
```
Zero failures, zero errors, zero dangling references — all three, not two out of three.

---

## Section 16: registry backend — minimal static surface

**Decision made:** static-file registry, no API server, matching Cargo's alternative-registry RFC
minimum surface (already cited as the target in `pata-production-readiness.md`). Concretely: a git
repository (or any static file host — the design doesn't care which) containing:
```
index/
  <package-name>/
    index.json   # array of {"version": "1.2.3", "checksum": "<sha256>", "url": "https://.../pkg-1.2.3.tar.gz"}
```
No publish command, no auth, no dynamic API in this pass — publishing to the index is a manual
`git commit` to the index repo (or equivalent) by a package author, exactly matching how a
brand-new, single-maintainer ecosystem actually starts (this is also, concretely, how the very
first versions of both Homebrew's and early Cargo's own registries worked before either had a
server). This is a deliberate, decided scope floor — a real API-backed registry (search, auth,
yanking, download counts) is future work once there's more than a handful of packages to justify
it, not blocked on any technical unknown.

**Files to touch:**
- `pata/package/Cargo.toml` — add `ureq = "2"` (a minimal blocking HTTP client — simpler than
  `reqwest` for a single synchronous GET, no async runtime dependency needed anywhere else in this
  crate).
- `pata/package/src/registry.rs` — **new file**.
- `pata/package/src/lib.rs` — add `pub mod registry;` and re-export.
- `pata/package/src/resolver.rs` — `available_versions` (added in Section 7) gains a registry
  fallback when no local `.pata-version` marker exists.
- `pata.toml`'s (this repo's own conceptual future) or any project's manifest — no new syntax
  needed; the registry index URL is a fixed constant for the first pass (see step 1 below), not
  configurable — configurability is itself a follow-up once there's ever a second registry to
  point at.

**Exact steps:**

1. In `pata/package/src/registry.rs`:
   ```rust
   //! Minimal static-file package registry client: a git-or-static-host-served JSON index, no
   //! API server. See docs/design/pata-production-readiness.md and
   //! docs/design/pata-implementation-spec.md Section 16 for the design rationale.

   use anyhow::{Context, Result};
   use serde::Deserialize;

   /// Fixed for this first pass — not yet configurable (see this section's own note on why).
   /// Replace with the real hosted index URL once one exists; this is a placeholder that must be
   /// updated before this feature is usable end-to-end, not before it compiles/tests.
   pub const DEFAULT_INDEX_BASE_URL: &str = "https://REPLACE-ME.example/pata-index";

   #[derive(Debug, Clone, Deserialize)]
   pub struct RegistryEntry {
       pub version: String,
       pub checksum: String,
       pub url: String,
   }

   /// Fetch and parse `<index_base>/<name>/index.json`. Returns an empty Vec (not an error) on a
   /// 404 — a package simply not being in the index is a normal "no versions available" case for
   /// Section 7's resolver to handle, not a network-error case.
   pub fn fetch_index(index_base: &str, name: &str) -> Result<Vec<RegistryEntry>> {
       let url = format!("{}/{}/index.json", index_base.trim_end_matches('/'), name);
       let resp = ureq::get(&url).call();
       match resp {
           Ok(r) => {
               let entries: Vec<RegistryEntry> = r
                   .into_json()
                   .with_context(|| format!("index batili kwa {name}"))?;
               Ok(entries)
           }
           Err(ureq::Error::Status(404, _)) => Ok(vec![]),
           Err(e) => Err(anyhow::anyhow!("imeshindwa kupata index ya '{name}': {e}")),
       }
   }

   /// Download the tarball at `entry.url` into `dest_dir` (creating it), verify its SHA-256
   /// against `entry.checksum`, and extract it. Uses `tar`/`flate2` for extraction — add both as
   /// new dependencies (`tar = "0.4"`, `flate2 = "1"`) if not already present in
   /// pata/package/Cargo.toml.
   pub fn fetch_and_verify(entry: &RegistryEntry, dest_dir: &std::path::Path) -> Result<()> {
       let resp = ureq::get(&entry.url)
           .call()
           .with_context(|| format!("imeshindwa kupakua {}", entry.url))?;
       let mut bytes = Vec::new();
       std::io::copy(&mut resp.into_reader(), &mut bytes)?;

       use sha2::{Digest, Sha256};
       let mut hasher = Sha256::new();
       hasher.update(&bytes);
       let actual = format!("{:x}", hasher.finalize());
       if actual != entry.checksum {
           anyhow::bail!(
               "hashi ya {} haiendani: inatarajiwa {}, imepatikana {}",
               entry.url, entry.checksum, actual
           );
       }

       std::fs::create_dir_all(dest_dir)?;
       let tar = flate2::read::GzDecoder::new(&bytes[..]);
       let mut archive = tar::Archive::new(tar);
       archive.unpack(dest_dir)?;
       Ok(())
   }
   ```

2. In `pata/package/src/resolver.rs`'s `available_versions` (from Section 7), add a fallback: when
   the local `.pata-version` marker is absent, call `crate::registry::fetch_index
   (registry::DEFAULT_INDEX_BASE_URL, name)` and map each `RegistryEntry` into a `semver::Version`
   (skip entries whose `version` field fails to parse, rather than erroring the whole resolve).

3. Add `pub mod registry;` to `pata/package/src/lib.rs` and re-export `RegistryEntry`,
   `fetch_index`, `fetch_and_verify`.

**This section produces working, testable code but is NOT usable end-to-end without a real hosted
index existing at some real URL** — `DEFAULT_INDEX_BASE_URL` is a placeholder by design (see the
constant's own doc comment above). That is the one genuinely non-mechanical piece left in this
entire spec: someone must actually stand up and host an index (even a static `git` repo on GitHub
Pages is sufficient) and update the constant — a hosting/ops decision, not a code-shape one, and
out of scope for whichever model executes this section.

**Acceptance check:**
```bash
cargo build -p pata-package 2>&1 | tail -30
cargo test -p pata-package 2>&1 | tail -30
```
Write tests for `fetch_index`/`fetch_and_verify` using a local mock: spin up a tiny local HTTP
server for the test only (the `tiny_http` crate, dev-dependency, is the simplest option — add
`tiny_http = "0.12"` under `[dev-dependencies]` in `pata/package/Cargo.toml`) serving a fixed
`index.json` response and a fixed tarball on `127.0.0.1:<ephemeral port>`, and assert
`fetch_index`/`fetch_and_verify` work against it correctly (including the checksum-mismatch error
path — serve a tarball that doesn't match a deliberately-wrong `checksum` field and assert
`fetch_and_verify` errors). Do not write a test that reaches the real network or the placeholder
`DEFAULT_INDEX_BASE_URL`.

---

## Section 17: Mwalimu (LSP) inlay hints

**Decision made:** the smallest, most mechanical of the four originally-bundled LSP items
(incremental resolution, inlay hints, code actions, DAP) — tackled first for that reason; the other
three remain genuinely large and are addressed after it below, not deferred past this doc.

**Files to touch:**
- `pata/lsp/src/lib.rs` — register the new LSP capability + request handler (find where
  `completion`/`hover` etc. are registered — `grep -n "fn initialize\|ServerCapabilities"
  pata/lsp/src/lib.rs` to find the capabilities-declaration site).
- `pata/lsp/src/inlay.rs` — **new file**.

**Exact steps:**

1. In `pata/lsp/src/lib.rs`, find `ServerCapabilities` construction (in the `initialize` handler)
   and add `inlay_hint_provider: Some(tower_lsp::lsp_types::OneOf::Left(true))` to it (check the
   exact existing field-list style around it first — likely other `Some(OneOf::Left(true))` fields
   already sit there for `hover_provider`/`definition_provider`, mirror that exactly).

2. Create `pata/lsp/src/inlay.rs`:
   ```rust
   //! Inlay hints: inferred `weka` binding types shown inline in the editor (e.g. `weka x = 1`
   //! renders with a ghost `: Namba` after `x`), and parameter-name hints at call sites (e.g.
   //! `jumla(2, 3)` renders `jumla(a: 2, b: 3)`).

   use asili_parser::{Expr, Module, Stmt};
   use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position};

   /// Type-inference for a `weka` binding's inlay hint: reuses whatever the semantic analyzer
   /// already infers (do not reimplement type inference here — call into
   /// asili_parser::semantic's existing analyzer output, the same one hover.rs already consumes
   /// for its own type-signature display; check pata/lsp/src/hover.rs's existing pattern for how
   /// it gets a binding's inferred type from the analyzer before writing this, and reuse that
   /// exact mechanism rather than inventing a second one).
   pub fn compute_inlay_hints(module: &Module, /* + whatever hover.rs's pattern needs */) -> Vec<InlayHint> {
       let mut hints = Vec::new();
       for func in &module.functions {
           collect_hints_in_block(&func.body, &mut hints);
       }
       hints
   }

   fn collect_hints_in_block(block: &asili_parser::Block, hints: &mut Vec<InlayHint>) {
       for stmt in &block.statements {
           if let Stmt::Let { ty: None, line, column, .. } = stmt {
               // ty: None means no explicit type annotation was written — this is exactly the
               // case an inlay hint should fill in. ty: Some(_) means the user already wrote it;
               // don't hint what's already visible.
               // TODO: resolve the binding's real inferred type via the semantic analyzer (see
               // doc comment above) and push an InlayHint at Position { line: *line as u32 - 1,
               // character: *column as u32 } with label InlayHintLabel::String(format!(": {}",
               // inferred_type_name)) and kind Some(InlayHintKind::TYPE).
           }
           // TODO: recurse into nested blocks (If/While/etc.) the same way Section 13's
           // check_block needed to — same open item, same fix shape, both should be resolved
           // together if executed in the same pass since they need the same Stmt-variant
           // enumeration work.
       }
   }
   ```

3. Wire an LSP request handler for `textDocument/inlayHint` in `pata/lsp/src/lib.rs` (or
   `server.rs`, wherever other request handlers like `hover`/`completion` are implemented — check
   that file's existing pattern for how a document's current parsed `Module` is fetched from
   `doc_store.rs` before writing this, and mirror it exactly) calling `inlay::compute_inlay_hints`.

**Acceptance check:**
```bash
cargo build -p pata-lsp 2>&1 | tail -30
cargo test -p pata-lsp 2>&1 | tail -30
```
Write at least one test that parses a small source string with an untyped `weka` binding, calls
`compute_inlay_hints`, and asserts a hint is returned at the expected position with a non-empty
label.

---

## Section 18: Mwalimu (LSP) code actions (quick-fixes)

**Decision made:** two concrete quick-fixes, chosen because both have an exact, unambiguous fix
(no judgment call about *what* the fix should be, only *whether* to offer it) — this is what makes
a code action mechanical rather than a design problem:
1. **"Ongeza `pata nadhifu`"** — offered on any formatting-violation diagnostic; applies the
   already-existing `pata_fmt::canonical_format` (done, per this doc's Section-0 note) to the whole
   document.
2. **"Ongeza `///` maelezo"** — offered on a missing-doc-comment diagnostic (the one
   `thibitisha`/`enforce_docs`-style check produces, mirrored here as an LSP-side diagnostic if one
   doesn't already exist as such — check `pata/lsp/src/diagnostics.rs` first); inserts a
   `/// TODO: eleza <name>.\n` line above the flagged item (this exact insertion already exists as
   dead code in `pata/lsp/src/lib.rs:420`, per this conversation's earlier `grep` finding
   `new_text: format!("{indent}# TODO: eleza {name}.\n")` — **reuse that exact line**, wiring it
   into a real `codeAction` response instead of wherever it currently sits unused).

**Files to touch:**
- `pata/lsp/src/lib.rs` — the existing dead-code TODO-insertion snippet at line 420 (confirm this
  line number is still accurate — file may have shifted since that earlier `grep`; re-run `grep -n
  "TODO: eleza" pata/lsp/src/lib.rs` first) — find what function currently contains it and why it's
  unused, before wiring it into a new handler.
- `pata/lsp/src/code_actions.rs` — **new file**.

**Exact steps:**

1. Register `code_action_provider: Some(CodeActionProviderCapability::Simple(true))` in
   `ServerCapabilities`, same location/pattern as Section 17 step 1.

2. Create `pata/lsp/src/code_actions.rs`:
   ```rust
   //! Quick-fix code actions offered on specific diagnostic codes.

   use tower_lsp::lsp_types::{CodeAction, CodeActionKind, Diagnostic as LspDiagnostic, TextEdit, WorkspaceEdit};
   use std::collections::HashMap;

   /// Given the diagnostics LSP already computed for a document (from diagnostics.rs's existing
   /// pipeline) and the document's current text/URI, return whichever quick-fixes apply.
   pub fn compute_code_actions(
       uri: &tower_lsp::lsp_types::Url,
       text: &str,
       diagnostics: &[LspDiagnostic],
   ) -> Vec<CodeAction> {
       let mut actions = Vec::new();

       // Fix 1: "Ongeza `pata nadhifu`" — offered whenever ANY diagnostic in this batch relates
       // to formatting (check diagnostics.rs for the exact code a formatting violation uses, if
       // the LSP even surfaces one today — it may not, since nadhifu is a CLI-only check; if no
       // such diagnostic code exists, this fix is instead offered unconditionally, always
       // available, not diagnostic-gated — confirm which case applies before implementing).
       let formatted = pata_fmt::canonical_format(text); // confirm pata-fmt is (or becomes) a pata-lsp dependency; if pata_fmt only has a binary target today, check whether it also exposes a lib.rs — if not, add one, mirroring how pata-package already works as both
       if formatted != text {
           actions.push(CodeAction {
               title: "Fomati faili (pata nadhifu)".to_string(),
               kind: Some(CodeActionKind::SOURCE_FIX_ALL),
               edit: Some(WorkspaceEdit {
                   changes: Some(HashMap::from([(
                       uri.clone(),
                       vec![TextEdit {
                           range: full_document_range(text),
                           new_text: formatted,
                       }],
                   )])),
                   ..Default::default()
               }),
               ..Default::default()
           });
       }

       // Fix 2: "Ongeza maelezo" — offered per missing-doc diagnostic.
       // TODO: for each diagnostic in `diagnostics` matching the missing-docs code, build a
       // CodeAction inserting the existing `format!("{indent}# TODO: eleza {name}.\n")` snippet
       // (found at pata/lsp/src/lib.rs's current TODO-insertion site) at the right position —
       // reuse that exact string-building logic rather than rewriting it.

       actions
   }

   fn full_document_range(text: &str) -> tower_lsp::lsp_types::Range {
       // Same computation as pata/lsp/src/format.rs's format_to_edits already does for its own
       // full-document replacement — reuse that exact logic (copy the range-computation lines,
       // same "avoid circular dependency" constraint as format.rs's own doc comment explains for
       // why it's a copy rather than a shared call).
       unimplemented!("copy format_to_edits's range computation from pata/lsp/src/format.rs")
   }
   ```

3. Wire a `textDocument/codeAction` request handler calling `code_actions::compute_code_actions`,
   same pattern-matching approach as Section 17 step 3.

**Acceptance check:**
```bash
cargo build -p pata-lsp 2>&1 | tail -30
cargo test -p pata-lsp 2>&1 | tail -30
```
One test: an unformatted source string, call `compute_code_actions`, assert one action titled
"Fomati faili (pata nadhifu)" is returned with a `WorkspaceEdit` whose replacement text equals
`pata_fmt::canonical_format`'s output.

---

## Section 19: Mwalimu (LSP) incremental re-resolution — DONE (as scoped below)

**Implemented:** `DocStore::diagnostics_for` (content-hash cache, `pata/lsp/src/doc_store.rs`)
and `workspace::affected_project_roots` (scoped `did_change_watched_files` invalidation,
`pata/lsp/src/workspace.rs`) — both wired into `did_open`/`did_change`/`did_change_watched_files`
in `pata/lsp/src/lib.rs`. Cross-file staleness is handled via an explicit `invalidate()` call in
`did_change_watched_files` before recomputing, per this section's own step 3, rather than the
`Module.imports`-based reverse-lookup originally sketched there — `did_change_watched_files`
already recomputes every open document unconditionally on any watched-file event (a pre-existing
behavior, not changed here), so the finer-grained "only B if it imports A" targeting sketched in
step 3 wasn't necessary to get the caching's actual benefit (skipping same-text recomputation).
Verified with 10 new tests (6 in `doc_store.rs` proving real cache hits/misses via a
`#[cfg(test)]` recompute counter per this section's own acceptance-check note, 4 in
`workspace.rs` proving scoped invalidation against real on-disk project directories).

**Decision made:** not a full salsa-style incremental-computation framework (pulling in the
`salsa` crate and restructuring the entire LSP around query-based recomputation is a rewrite, not
an incremental improvement, and risks the same "more than this fix needs" trap `workspace.rs`'s own
comment already warned about once). Instead: **per-file re-check caching keyed by content hash** —
the smallest change that removes the actual, measured cost (full-workspace re-check on every
keystroke), without a framework migration. On `didChange`, only the changed document is
re-lexed/re-parsed/re-analyzed; other documents' previously-computed diagnostics/symbols are
reused from a cache unless *their* dependency graph includes the changed file (via `leta`), in
which case they're invalidated and recomputed lazily on next access, not eagerly on every edit.

**Files to touch:**
- `pata/lsp/src/doc_store.rs` — add a per-document cache entry (parsed `Module` + diagnostics +
  content hash).
- `pata/lsp/src/lib.rs` (or `server.rs`) — the `didChange` handler.

**Exact steps:**

1. `Read` `doc_store.rs` in full first (not fully read during this conversation's exploration —
   only its role, "in-memory URI → text map," was confirmed from `mwalimu-design.md`, not its
   actual struct layout). Add a cache field alongside the existing text map:
   ```rust
   pub struct CachedAnalysis {
       pub content_hash: u64, // same DefaultHasher approach as interface_registry.rs's own fingerprint()
       pub module: asili_parser::Module,
       pub diagnostics: Vec<tower_lsp::lsp_types::Diagnostic>,
   }
   ```
   Add `HashMap<Url, CachedAnalysis>` to whatever struct `doc_store.rs` currently exposes (its
   exact name wasn't confirmed during this conversation — read the file to get it right, do not
   guess).

2. In the `didChange` handler, before re-lexing/re-parsing/re-analyzing a document: compute the new
   text's content hash and compare against the cached entry's `content_hash`. If equal (can happen
   on a no-op edit event, e.g. cursor-only movement some editors still fire changes for), skip
   recomputation entirely and republish the cached diagnostics unchanged.

3. For cross-file invalidation: when document A changes, find every cached document B whose
   `Module.imports` (already parsed and cached from B's own last analysis) names A's module — this
   requires knowing each document's resolved module name, which `workspace.rs`'s existing
   `WorkspaceIndex` (or its Section-15 replacement, `pata_core::resolve`, if Section 15 was
   executed first — **check which is present before implementing this step**, since Section 15
   deletes `workspace.rs`) already computes. Invalidate (remove from cache) every such B; do not
   eagerly recompute them — let the next `textDocument/hover`/`didOpen`/etc. request on B trigger a
   fresh computation on demand (this "lazy on next access" behavior is the actual incrementality
   win: an edit to A no longer forces immediate re-analysis of every file that imports it, only
   marks them stale).

**Acceptance check:**
```bash
cargo build -p pata-lsp 2>&1 | tail -30
cargo test -p pata-lsp 2>&1 | tail -30
```
Write a test that: opens two documents where B imports A, triggers a `didChange` on B only,
and asserts (via whatever internal accessor the cache exposes, or by checking a counter/mock
incremented only on actual recomputation — add a `#[cfg(test)]`-only recompute-counter field to
`CachedAnalysis`'s container if there's no other way to observe "was A actually re-analyzed") that
A's cached analysis was NOT recomputed as a side effect of B's change.

---

## Section 20: DAP (Debug Adapter Protocol server) — DONE (protocol layer; see dap-later.md)

**Implemented:** `pata/dap` (new crate, lib `pata_dap` + bin `pata-dap`), depending on the real
`dap = "=0.4.1-alpha1"` crate (confirmed current via the crates.io API — no stable release exists
for this crate; pinned the exact alpha rather than assuming `0.4`). `hook.rs` defines the
`DebugHook` trait exactly as specified below; `mock_hook.rs` is a real (not `#[cfg(test)]`-gated)
fake with genuine thread-blocking pause/resume (a `Mutex`+`Condvar`, not a stub); `server.rs`
implements `initialize`/`launch`/`setBreakpoints`/`configurationDone`/`continue`/`stackTrace`/
`scopes`/`variables`/`threads`/`disconnect` request handling plus the stdio poll loop, using the
`dap` crate's own `Request::success`/`.ack()`/`.error()` helpers rather than reimplementing them.
Verified with 14 tests (`cargo test -p pata-dap`) including one exercising the full stdio loop
against real `Content-Length`-framed wire bytes, and manually against the compiled binary over a
live pipe. See `docs/design/dap-later.md` for the up-to-date status write-up (that file, not this
one, is the canonical "what works today" reference per the wiki-sync table).

**Decision made:** a **new binary crate** `pata/dap`, not a module inside `pata-lsp` — DAP and LSP
are structurally unrelated protocols (different message shapes, different lifecycle, different
transport conventions in practice even though both often run over stdio), and `pata-lsp`'s own
`dap-later.md` design placeholder (referenced but not read in full during this conversation —
`Read` it in full before starting this section, since it may already contain real design decisions
that supersede parts of this write-up) already frames DAP as a separate concern. Scope for this
first pass: the minimum viable DAP surface — `launch`, `setBreakpoints`, `continue`, `stackTrace`,
`variables` — enough to single-step through a running `.as` program in an editor, not the full DAP
spec (no `stepIn`/`stepOut`/watch expressions/conditional breakpoints in this pass).

**This section requires evaluator-level support that does not exist yet** (a way to pause execution
at a given line and inspect the current environment) — unlike every other section in this spec,
this one is **not purely additive to `pata/`**; it needs a new hook in `core/evaluator` first. Given
this doc's established scope boundary (`pata/` toolchain, `core/` treated as a given —
`pata-production-readiness.md`'s own stated scope), implementing this fully is **out of bounds for
this doc**. What follows instead is the exact interface contract the evaluator-side work must
satisfy, so that whoever does the `core/evaluator` work (a separate task, likely needing its own
spec at that layer) has an unambiguous target, and the `pata/dap` crate itself can be built and
tested against a mock implementing that contract without waiting on the real one.

**Contract `core/evaluator` must eventually provide** (do not implement in `core/evaluator` as part
of this section — only depend on this shape existing):
```rust
pub trait DebugHook {
    /// Called before executing the statement at `line`. Returning `true` pauses execution and
    /// blocks (e.g. on a channel recv) until `resume()` is called from another thread.
    fn should_pause(&self, line: usize) -> bool;
    fn resume(&self);
    /// Snapshot of currently-in-scope bindings, for a `variables` DAP request.
    fn current_bindings(&self) -> Vec<(String, String)>; // (name, debug-formatted value)
}
```

**Files to touch (buildable/testable now, against a mock `DebugHook`):**
- `pata/dap/Cargo.toml` — new crate, binary target `pata-dap`. Depends on `dap` crate (crates.io
  `dap = "0.4"` — a Rust DAP-protocol-types library; confirm current version on crates.io before
  pinning, do not assume `0.4` is still current) for message (de)serialization, matching how
  `pata-lsp` uses `tower-lsp` for the equivalent LSP role.
- `pata/dap/src/main.rs`, `pata/dap/src/server.rs` — stdio transport + request dispatch, modeled
  directly on `pata/lsp/src/server.rs`'s existing structure (read that file for the pattern before
  writing this crate — same request/response loop shape, different message types).
- `pata/dap/src/mock_hook.rs` — a `#[cfg(test)]`-only or `dev-dependencies`-only fake `DebugHook`
  implementation, enough to test `pata-dap`'s own protocol handling in isolation from the real
  evaluator (which doesn't implement the trait yet).
- `pata/cli/src/commands/` — no new command needed; DAP servers are conventionally launched
  directly by the editor's debug-adapter configuration pointing at the `pata-dap` binary, not via
  `pata <subcommand>` (matching how `pata-lsp`'s standalone binary coexists with `pata mwalimu` —
  a `pata dibagi`-style CLI wrapper is optional follow-up, not required for this section).

**Acceptance check:**
```bash
cargo build -p pata-dap 2>&1 | tail -30
cargo test -p pata-dap 2>&1 | tail -30
```
Tests exercise `pata-dap`'s request/response handling against `mock_hook.rs`, proving the protocol
layer works — they cannot prove real debugging works end-to-end until `core/evaluator` implements
`DebugHook` for real, which is explicitly out of this section's scope. Note this limitation in the
commit message for this section, so it isn't later mistaken for a finished, usable debugger.

---

Every item originally listed as "Deferred" now has a decided design and a spec (Sections 10-20).
The only genuinely unresolvable-by-this-doc residue is Section 16's placeholder index URL (a
hosting/ops action, not a code-shape decision) and Section 20's evaluator-side `DebugHook`
implementation (out of this doc's `pata/`-only scope by the production-readiness doc's own stated
boundary — a `core/evaluator` task, not a `pata/` one). Both are called out explicitly at their own
section rather than left as an unmarked gap.
