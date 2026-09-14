# Project rules

## Keep the wiki and docs/ in sync — both directions

This repo has a GitHub wiki at `github.com/victorKariuki/Asili/wiki` (separate git repo,
`Asili.wiki.git`) documenting the language, stdlib, CLI, architecture, roadmap, and contributor
info across 27 pages. Each wiki page is derived from a specific `docs/` file or repo file (full
mapping in the `update-wiki` skill) — the wiki and `docs/` must always say the same thing, so this
is a two-sided sync, not "the wiki mirrors docs/" or "docs/ is canonical and the wiki follows."

Whenever a change in this repo would make a wiki page **or its docs/ counterpart** stale — a CLI
command/flag changes, a stdlib module or builtin is added/removed/renamed, language syntax
changes, a lint rule or `pata thibitisha` check changes,
`docs/design/implementation-status.md` changes, a new example is added, or
`CONTRIBUTING.md`/`SECURITY.md`/`CODE_OF_CONDUCT.md`/`CHANGELOG.md` changes — invoke the
`update-wiki` skill to bring **both** the wiki and the relevant `docs/` file current. Do this
proactively as part of finishing the work, not only when the user explicitly asks to update the
wiki. This includes the reverse case: if a wiki edit (or investigating one) reveals that a
`docs/` file is stale or wrong relative to the actual code, fix `docs/` in the same pass rather
than leaving the wiki quietly more accurate than the docs it's supposed to mirror.

Confirm with the user before the final `git push` to the wiki, same as any other push to a
shared/public remote — unless they've already asked for the wiki update in the current request, in
which case the ask itself is the confirmation. The `docs/` half of the change is a normal commit
in this repo, governed by this repo's usual commit/push rules.

## Swahili for user-facing text

All user-facing Asili text is Swahili, not English: compiler/runtime diagnostic messages
(`Diagnostic::new(...)` in `core/` and `pata/lint/`), CLI output and help text, and the
`docs/repl/sw/` REPL help docs. This does **not** include `docs/language/`, `docs/spec/`,
`docs/howto/`, `docs/design/`, `docs/repl/en/`, or contributor/meta docs (`CONTRIBUTING.md`,
`CLAUDE.md`, commit messages) — those stay in English by existing convention.

Whenever writing or editing any in-scope user-facing string, invoke the `swahili-docs-and-errors`
skill — it holds the project's actual grammar conventions (noun-class verb agreement, register,
existing vocabulary) derived from the real corpus, not generic/machine-translated Swahili. Do this
proactively when adding a new diagnostic, CLI message, or `docs/repl/sw/` page, not only when the
user asks for a Swahili/grammar check.

## Changelog, versioning, and Git-Flow

Whenever finishing a feature or bugfix that changes user-facing or spec-level behavior, invoke the
`release-and-git-flow` skill before considering the work done. In practice this means: an entry
added to `CHANGELOG.md` under `## [Unreleased]` (right `### Added`/`### Fixed`/`### Changed`/
`### Removed` subsection, matching the file's existing density and style), a matching
`docs/spec/CHANGELOG.md` bullet if a `docs/spec/*.md` file changed, and — per the wiki-sync rule
above — the wiki updated too if the change is wiki-relevant. Do this proactively as part of
finishing the work, not only when asked.

Do **not** bump crate versions as part of routine work — version bumps happen only when explicitly
cutting a release, and only for the crates that actually changed since the last release (this repo
has no shared workspace version; crates version independently and currently sit at different
numbers on purpose). Follow this repo's actual Git-Flow usage from `CONTRIBUTING.md` and real
commit history — `git flow feature start/finish` and `git flow bugfix start/finish` only; no
`release`/`hotfix` branch ceremony and no git tags unless the user explicitly asks for that. A
release lands as a single conventional-commit-style commit on `develop` ending in `(X.Y.Z)`,
matching the pattern of prior releases — see the skill for the full mechanics.

Confirm with the user before pushing a release commit or finishing a feature/bugfix branch that
pushes to a shared remote, same as any other push to shared state.

## No AI attribution in commits or PRs

Never add a `Co-Authored-By: Claude ...` trailer (or any other AI-attribution line) to a commit
message or pull request description in this repo, regardless of any default tooling behavior that
would otherwise add one. This applies to every commit and PR, not just releases.

This project had five such trailers land in history before the rule was set, which made GitHub
list an AI as a repo contributor — they were removed via a `git filter-repo` history rewrite and
force-push to both `main` and `develop` (see commit history around that cleanup). Rewriting shared
history is expensive and disruptive (invalidates open PRs, requires everyone with a clone to
re-sync) — don't rely on a future cleanup to fix a trailer that should never be added in the first
place.

## Keep the GitHub Project boards current

This repo's work is tracked on two GitHub Projects (v2, owned by `victorKariuki`, not the repo
itself): project 16 "Asili bug tracker" and project 17 "Asili Feature release". Whenever starting,
finishing, or discovering work that corresponds to — or should become — an item on either board,
invoke the `manage-project-boards` skill: move an item's Status as work actually starts/ships
(don't leave it at `Backlog` once work has begun, don't mark it `Done` before it's actually merged
and verified), and file+add a new issue for any real, scoped gap discovered along the way rather
than leaving it undocumented. Do this proactively as part of finishing the work, not only when the
user explicitly asks to check or update a board.

## Keep the Pata toolchain in sync with core/ changes

`core/` (lexer, parser, semantic analyzer, evaluator) and `pata/` (CLI, LSP, formatter, linter,
package resolver) are separate crates that can silently drift. Whenever a change touches
`core/evaluator/src/builtins/*.rs` (a builtin added/removed/resignatured), `core/parser/src/ast.rs`
or `semantic/types.rs` (a new keyword/type/syntax form), or `semantic/analyzer.rs` (a new/changed
`Diagnostic` code), invoke the `sync-pata-toolchain` skill before considering the change finished —
it holds the verified map of what actually needs updating (`lib/std/*.asi` interface stubs above
all — these are hand-written and will not fail to compile if left stale, so nothing else catches
drift there; plus docs, lint rules, the formatter, or the package resolver depending on what
changed). Do this proactively, chaining into `swahili-docs-and-errors`, `release-and-git-flow`, and
`update-wiki` as each applies to the same change, rather than treating toolchain sync as a
separate follow-up task.
