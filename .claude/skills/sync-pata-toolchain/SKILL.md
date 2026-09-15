---
name: sync-pata-toolchain
description: After changing core/ (lexer, parser, semantic analyzer, evaluator, builtins), check whether the Pata toolchain needs matching updates — lib/std/*.asi interface stubs, pata-lint's rule set, pata-fmt, pata-package, pata-lsp, and the CLI commands themselves. Use whenever a change touches core/evaluator/src/builtins/*.rs, core/parser/src/ast.rs, core/parser/src/semantic/*.rs, adds/removes a language keyword or stdlib function, or changes a Diagnostic code — before considering that change finished.
---

# Keeping the Pata toolchain in sync with core/ changes

`core/` (the language itself — lexer, parser, semantic analyzer, evaluator) and `pata/` (the
toolchain built on top of it — CLI, LSP, formatter, linter, package resolver) are separate crates
that can drift. This project has already documented one concrete coupling in `CONTRIBUTING.md`
("keep `lib/std/` in sync with built-in modules and `builtin_modules.rs`") — this skill covers
that plus the rest of the toolchain surface that can silently go stale the same way.

## What's actually coupled to `core/` (verified against source, not assumed)

| If you changed... | Check... | Why |
|---|---|---|
| `core/evaluator/src/builtins/*.rs` (a builtin function added/removed/resignatured) | `lib/std/*.asi` — the matching module's interface stub | `.asi` files are **hand-written** signature-only stubs (no shared codegen) — e.g. `lib/std/faili.asi` lists `soma_faili(njia: Neno) -> Tokeo<Neno, Neno>` by hand. `pata/cli/src/pipeline/builtin_modules.rs` itself is just `pub use asili_parser::builtins::*;` (a re-export, not a second copy — it can't drift on its own), but the `.asi` stubs are real independent text that will drift if not updated by hand. Per `CONTRIBUTING.md`. |
| `core/parser/src/ast.rs` or `core/parser/src/semantic/types.rs` (a new type, keyword, or syntax form) | `docs/language/*.md`, `docs/repl/{en,sw}/*.md`, the wiki (`Language-Basics`/`Data-Structures`/`Type-System` pages via `update-wiki`) | Tutorial and REPL docs describe syntax by hand-written example — a new keyword/type isn't self-documenting anywhere else. |
| `core/parser/src/semantic/analyzer.rs` (a new/changed `Diagnostic::new("SEMxxx", ...)`) | Any doc that enumerates diagnostic codes; the message text itself per the `swahili-docs-and-errors` skill | New error codes should read as correct Swahili (noun-class agreement etc.) on introduction, not as a later cleanup pass. |
| Any of the above, in a way a user would notice | `CHANGELOG.md` under `## [Unreleased]`, per the `release-and-git-flow` skill | Standing project rule — every user-facing change gets a changelog entry. |
| Any of the above, in a way the wiki documents | The matching wiki page(s), per the `update-wiki` skill | Standing project rule — wiki and docs/ must agree with each other and with the code. |
| A new lint-worthy pattern introduced by a language change (e.g. a new footgun the semantic analyzer now allows) | `pata/lint/src/rules/*.rs` — does an existing or new `LINTxxx` rule need to cover it? | `pata-lint` does **not** hardcode builtin/module names (verified: no `builtins::`/module-name references in `pata/lint/src/*.rs`) — its rules are structural, so it usually doesn't need touching for a builtin change, but a new *syntax form or semantic allowance* can open a gap a lint rule should close. |
| A new type or value kind the formatter needs to lay out specially | `pata/fmt/src/format.rs` | The formatter is token/line-based (`README.md`: "best-effort") — new syntax with unusual layout needs (multi-line literals, new bracket pairs) may need explicit handling. |
| A change to `pata.toml` schema (including `[eneo-kazi]` workspaces) or dependency resolution | `pata/package/src/*`, `docs/design/package-manager-design.md`, wiki's `Package-Management` page | `pata.toml` is the one manifest file/format for both leaf and workspace-root projects (real TOML parsing in `pata-cli`'s `load_project_config`) — independent of `core/`, but its own schema changes need the same doc/wiki sync as anything else user-facing. |

`pata-lsp` does **not** need manual builtin-name updates — it consumes builtin/module tables
through the same `asili_parser::builtins` re-export the CLI uses (verified:
`pata/lsp/src/symbols.rs` and `semantic.rs` go through the shared table, not a hardcoded copy).
LSP changes are only needed for *new capability surface* (e.g. a new AST node needing its own
hover/goto-def handling), not for routine builtin additions.

## Workflow

1. **After any `core/` change, ask: does this add/remove/resignature a builtin, keyword, type, or
   diagnostic code?** If no (pure refactor, internal-only), stop here — nothing in `pata/` needs
   touching.
2. **If yes, walk the table above** for the specific kind of change and check/update each affected
   file. Don't assume a category is unaffected without checking — e.g. a builtin rename affects
   `.asi` stubs even if the function's behavior is unchanged.
3. **For `lib/std/*.asi` specifically:** find the matching module file (name matches the builtin
   module, e.g. `core/evaluator/src/builtins/faili.rs` ↔ `lib/std/faili.asi`), and hand-update the
   function/const signature list to match. These stubs carry doc comments and type signatures
   only — no bodies — matching the real builtin's parameter/return types exactly (`Neno`, `Namba`,
   `Tokeo<T,E>`, etc.).
4. **Chain into the other standing skills as needed** — `swahili-docs-and-errors` for any new
   user-facing string, `release-and-git-flow` for the changelog entry, `update-wiki` for the wiki
   + docs/ pages. Don't treat these as separate follow-up tasks; do them as part of finishing the
   same change.
5. **Build and test** (`cargo build`, `cargo test`) after touching `.asi` stubs or any `pata/`
   crate — a stub/signature mismatch won't necessarily fail to compile (they're not
   type-checked against `core/` at build time) but a stale stub is a real bug worth catching by
   eye during review, since nothing else will catch it.

## When NOT to touch the toolchain

- Pure internal refactors in `core/` with no change to any public builtin signature, keyword,
  type, or diagnostic code/message.
- Performance-only changes with no behavior change.
- Test-only changes.
