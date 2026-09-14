---
name: release-and-git-flow
description: Update CHANGELOG.md (and docs/spec/CHANGELOG.md when relevant), bump the right crate version(s), and commit/branch following this project's actual Git-Flow usage. Use whenever finishing a feature/bugfix that should be recorded in the changelog, when the user says "bump the version", "cut a release", "update the changelog", or when starting/finishing work that CONTRIBUTING.md's workflow governs (feature/bugfix branches, PRs to develop/main).
---

# Changelog, versioning, and Git-Flow for Asili

This project's real practice is narrower than textbook Git-Flow and textbook semver-per-repo — follow what's actually done here (verified against `CHANGELOG.md`, `docs/spec/CHANGELOG.md`, `Cargo.toml` files, and real commit history), not a generic convention.

## Facts about this repo's versioning (verified, don't assume otherwise)

- **No workspace-level shared version.** Each crate in the Cargo workspace (`core/*`, `pata/*`, `driver/wasm`) has its own independent `version` in its own `Cargo.toml`. As of the last check they legitimately differ (`pata-cli`/`pata-lsp`/`core/parser`/`core/evaluator` at `0.3.0`; `pata-fmt`/`pata-lint`/`core/diagnostics`/`core/lexer`/`driver/wasm` at `0.2.0`; `pata-package`/`pata-runner` at `0.2.1`) — this is **not drift to fix**, it reflects which crates actually changed in the last release. Don't force them to match.
- **No git tags are used** for releases. The record of what shipped in a version is `CHANGELOG.md`'s dated section headers, not `git tag -l` (currently empty) or GitHub Releases.
- **Releases land as a single commit on `develop`** (see `487fde8 feat: ... (0.3.0)`) — the pattern is a normal conventional-commit message with the version in parentheses at the end, not a `git flow release start/finish` ceremony. `CONTRIBUTING.md` itself only documents `feature`/`bugfix` branch types — it doesn't mention `release`/`hotfix`, so don't invent that ceremony unless the user asks for it.
- **Two separate changelogs, two separate scopes:**
  - `CHANGELOG.md` (repo root) — the implementation/tooling changelog. [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format: `## [Unreleased]` accumulates entries under `### Added`/`### Fixed`/`### Changed`/`### Removed` subheadings; on release, `[Unreleased]` is retitled to `## [X.Y.Z]` (matching whichever crate(s) actually bumped — usually the highest bump among touched crates, e.g. `pata-cli`'s version, since that's the user-facing version `pata --toleo` effectively tracks) and a fresh empty `## [Unreleased]` is added above it.
  - `docs/spec/CHANGELOG.md` — the **specification** changelog, only for changes to `docs/spec/*.md` files (language/type-system/stdlib-contract/tooling-contract semantics). Uses `## Unreleased` → dated section headers (no version numbers — the spec doesn't ship as a versioned crate). Only touch this file when a `docs/spec/` file actually changed.
  - A single piece of work often touches only one of these two files. Don't add a spec-changelog entry for a pure implementation change with no spec-level semantic change, and vice versa.
- **`pata.toml`'s `toleo` field is a per-project app version**, unrelated to crate versions — don't confuse the two. It defaults to `"0.1.0"` for new projects (`pata njozi`) and is the end user's own project's version, not Asili's.

## Workflow: finishing a unit of work

1. **Determine what actually changed:** run `git diff` / `git status` against the base branch, and check which crates under `core/`, `pata/`, `driver/` had source changes (not just `Cargo.lock`).
2. **Update `CHANGELOG.md`:** add entry/entries under `## [Unreleased]`, in the correct `### Added`/`### Changed`/`### Fixed`/`### Removed` subsection (create the subsection if this is the first entry of that kind since the last release). Match the existing style — dense, specific, names real functions/flags/files, explains *why* not just *what* (see any existing `[0.3.0]` entry for the bar to hit).
3. **If a `docs/spec/*.md` file changed with this work**, add a matching bullet to `docs/spec/CHANGELOG.md` under `## Unreleased`, referencing the specific spec file(s) and section(s) touched (match the existing bullet style: `- Topic ([file.md](file.md)): what changed.`).
4. **If this change should also update the wiki** (see the `update-wiki` skill — most user-facing changes do), do that in the same pass rather than as a separate follow-up.
5. **Do not bump a crate version for routine work.** Version bumps happen at release time (step below), not per-commit/per-PR — an in-progress `[Unreleased]` section is expected to accumulate multiple entries before a release.
6. **Commit** with a clear, conventional message (`feat:`/`fix:`/`docs:`/`chore:` prefix, matching real history — check `git log --oneline -20` for the exact style in use if unsure).

## Workflow: cutting a release

Only do this when the user explicitly asks to cut/tag a release or bump the version — not automatically at the end of routine work.

1. **Decide the version number** using semver against the highest-bumping crate's change (a breaking change → major, new backward-compatible feature → minor, fix-only → patch). Ask the user if it's ambiguous whether a change is breaking.
2. **Bump `version` in the `Cargo.toml`(s) of every crate that actually changed** since the last release — not every crate in the workspace. Check each candidate crate's real diff since the last release commit, don't bump reflexively.
3. **Retitle `CHANGELOG.md`'s `## [Unreleased]`** to `## [X.Y.Z]`, and add a fresh empty `## [Unreleased]` section above it (keeping the `### Added`/`### Fixed`/etc. structure ready for the next cycle, or omit empty subheadings until they have content — match what the file currently does at each prior release boundary).
4. **If `docs/spec/CHANGELOG.md` has unreleased content**, decide with the user whether it's tied to this same release or tracks independently (the two files have historically moved somewhat independently — check whether past releases retitled both files' `Unreleased` sections in the same commit or not, via `git log -p -- docs/spec/CHANGELOG.md CHANGELOG.md`, before assuming they always move together).
5. **Run `cargo test`** (and the LSP/CLI-specific test commands from `CONTRIBUTING.md`) before committing the release — a version bump commit should be on green tests.
6. **Commit** on `develop` with the established message pattern: `<type>: <summary of what shipped> (<version>)`, e.g. `feat: Faili/Mkondo/Kumbukumbu, real trait completeness, Seti, Namba_Kuu/Sahihi, concurrency (0.3.0)`.
7. **No `git flow release start/finish` and no `git tag`** unless the user explicitly asks for that ceremony — it's not what this repo's history shows being used. If the user does ask for it, `git flow release start X.Y.Z`, do the version-bump/changelog commit(s) on the release branch, then `git flow release finish X.Y.Z` (merges to both `main` and `develop`, tags `main`).
8. **Confirm before pushing** — a release commit lands on `develop` (a shared branch); follow the same push-confirmation rule as any other shared-branch push.

## Git-Flow for regular feature/bugfix work (per CONTRIBUTING.md — follow exactly, don't extend)

```bash
git flow feature start <name>      # branches off develop
# ...commit work...
git flow feature finish <name>     # merges back to develop, deletes feature branch
```

Same pattern for `bugfix` in place of `feature`. `CONTRIBUTING.md` documents only these two
branch types plus PRs against `main` (releases) or `develop` (everything else) — don't introduce
`hotfix`/`support`/`release` branch ceremony unless the user asks for it or a real need arises
(e.g. an urgent fix needed directly against a already-shipped `main` state).

- Never `git flow feature finish` with uncommitted changes outstanding — commit or stash first.
- Never force-push a feature branch others might have pulled without asking.
- A feature/bugfix branch should have its `CHANGELOG.md` (and `docs/spec/CHANGELOG.md` if
  applicable) entries added *before* `git flow feature finish`, as part of the branch's own
  commits — not bolted on afterward on `develop`.

## Self-check before calling a unit of work done

- Does `CHANGELOG.md` have an entry for this change, in the right subsection, at the right
  density (real names, real behavior, real "why")?
- If `docs/spec/` changed, does `docs/spec/CHANGELOG.md` have a matching bullet?
- If this is a release (not routine work), did every crate that actually changed get its version
  bumped, and only those crates?
- Does the commit message match this repo's real conventions (check recent `git log`, not a
  generic template)?
