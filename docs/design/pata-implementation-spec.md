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

## Deferred — NOT in this spec (too large/judgment-heavy for mechanical execution)

Listed here so nothing is silently dropped, per `pata-production-readiness.md`'s full item list.
These need a human or a higher-reasoning-effort model to make real design decisions before they
could be turned into a spec this exact:

- **Section 0 (shared `pata-core` crate extraction)** — structural refactor touching test
  infrastructure across 2 crates; the formatter work already landing directly in `pata/fmt`
  (rather than a shared crate) means this item's original payoff (avoid building the formatter
  twice) no longer applies — only the *resolver* reuse case for the LSP remains, which shrinks
  this item's scope. Needs re-scoping before it's spec-able.
- **Real dependency resolver with semver constraint solving + registry backend** — needs a design
  doc of its own (already flagged as such in the production-readiness doc).
- **`Workspace`/`Asili.toml` wiring into `pata jenga`** — blocked on an open question (does
  `pata.toml` gain its own `[workspace]` section, or does `Asili.toml` stay separate) that only
  the maintainer can resolve.
- **`pata thibitisha` type-stability check** — blocked on Section 1 (this spec) landing first, but
  even then needs the git-tag-baseline diffing logic designed, not just mechanically written.
- **`pata jaribu` parallel execution, timeouts, fixtures, coverage instrumentation** — each is a
  real concurrency/design task, not mechanical.
- **Mwalimu (LSP) incremental resolution, inlay hints, code actions, DAP** — all large, all need
  real design work.
- **New `pata-lint` rule categories** (unused vars, shadowing, unreachable code, redundant match
  arms) — each needs its own detection-logic design, unlike Section 4 above (which only adds tests
  for rules that already work).

If Haiku finishes Sections 1-5 cleanly, the next step is a higher-reasoning-effort pass to design
one of the deferred items down to this same level of exactness — not to attempt them directly from
the production-readiness doc's higher-level description.
