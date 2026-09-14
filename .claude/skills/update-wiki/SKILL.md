---
name: update-wiki
description: Update the Asili GitHub wiki (github.com/victorKariuki/Asili/wiki) to reflect a change just made in this repo, AND keep the matching docs/ file(s) in step — this is a two-sided sync, not a one-way copy. Use whenever a change touches something the wiki documents — CLI commands/flags (pata/cli/commands/*.md, pata/cli/src/commands/*.rs), stdlib modules/builtins (core/evaluator/src/builtins/*.rs), language syntax/keywords, LSP/Mwalimu features, the linter's rule set (pata/lint/src/rules/*.rs), the formatter, package management, examples/*, the roadmap/implementation status, or CONTRIBUTING.md/SECURITY.md/CODE_OF_CONDUCT.md/CHANGELOG.md. Also use when the user says "update the wiki" or "sync the wiki", or when editing a wiki page directly reveals that docs/ is stale.
---

# Update the Asili wiki — and keep docs/ in step

The wiki is a **separate git repository** from this project — `github.com/victorKariuki/Asili.wiki.git` — not a folder inside this repo. It is not cloned anywhere persistent by default; clone it fresh into the scratchpad each time.

**This is a two-sided sync.** Every wiki page (except FAQ, which is synthesized) has a specific
`docs/` file or repo file it's derived from — see the table below. Whichever side changed first,
bring the other side current in the same pass:

- **Repo change → wiki stale:** the common case (a CLI flag changes, a lint rule is added, a
  stdlib builtin is renamed). Update the wiki page(s) per the workflow below.
- **Wiki edited directly (via GitHub web UI) → docs/ may now be stale or may have been the thing
  that was actually wrong.** Before overwriting the wiki page from `docs/`, read what changed on
  the wiki side — if it corrected a real inaccuracy in `docs/` (matching real source-code
  behavior), port that correction back into the `docs/` file too, don't discard it.
- **A discrepancy surfaces between `docs/` and actual code while updating either side** — this has
  already happened four times (see precedents below) — fix the stale `docs/` file in the same
  pass, not just the wiki. Don't let the wiki quietly become more accurate than `docs/`; they
  should say the same thing, and both should match the code.

## The 27 pages, what each owns, and its docs/ counterpart

The right column is the file (or files) to update **in the same pass** — never leave the wiki and
`docs/` disagreeing about the same fact.

| Page | Owns | docs/ counterpart to keep in sync |
|---|---|---|
| Home | Landing page, pitch, quick start, full nav (mirrors `_Sidebar.md`) | `README.md` |
| Getting-Started | Install, build, `pata njozi`, project layout, first run | `docs/howto/00-getting-started.md` |
| Examples | Table of every `examples/*` program, what it demonstrates, how to run it | `examples/*/README.md` |
| Language-Tour | Swahili→English keyword cheat-sheet, index to the 4 language pages | `docs/language/README.md`, `docs/spec/03-syntax.md` |
| Language-Basics | Variables, operators, control flow, functions | `docs/language/01-misingi.md`, `02-kazi.md`, `03-udhibiti.md`, `09-waendeshaji.md` |
| Data-Structures | Orodha, Kamusi, Seti, Jozi, structs (`umbo`/`shughuli ya`), `jenum` | `docs/language/04-muundo-data.md` |
| Type-System | Numeric tiers (Urahisi/Nguvu/Ukubwa), nullability, casting, Sifa/generics | `docs/language/07-mfumo-wa-aina.md`, `08-njia-za-aina.md`, `docs/spec/04-type-system.md` |
| Error-Handling | Chaguo/Tokeo, `?`, `jaribu`, panics | `docs/language/05-makosa.md`, `docs/spec/08-resolved-decisions.md` §9.8 |
| Standard-Library | Module table: msingi, mfumo, majira, matumizi, faili, hisabati, runtime, syscall, kiungo, sambamba | `docs/spec/05-standard-library.md`, `README.md`'s stdlib table |
| Concurrency-and-Async | `tenda`/`njia`/`fungo` (implemented), `sawia`/`subiri` (not implemented) | `docs/design/concurrency-design.md`, `concurrency-async-design.md` |
| CLI-Reference | Every `pata` subcommand, flags, examples — **update whenever a CLI flag changes** | `pata/cli/commands/*.md`, `docs/howto/0{1,3,4,5,6}-*.md`, `README.md`'s CLI table |
| Editor-Support | Mwalimu (LSP) capabilities, VS Code extension, REPL help | `docs/design/mwalimu-design.md`, `docs/howto/02-use-lsp.md`, `06-use-repl.md`, `extensions/vscode/README.md` |
| Formatting-and-Linting | `pata nadhifu`, `pata-lint`'s rule set, `pata thibitisha`'s checks — **update whenever a lint rule or thibitisha check is added/removed** | `docs/howto/03-format-code.md`, `04-validate-docs.md`, `pata/cli/commands/thibitisha.md` |
| Package-Management | `pata.toml` schema, `pata ongeza`, path/vendored deps, lockfile | `docs/design/package-manager-design.md`, `docs/spec/06-tooling-and-ecosystem.md`, `docs/howto/05-add-dependency.md` |
| Architecture | Workspace/crate layout, execution pipeline, file/artifact conventions | `docs/spec/02-architecture-and-files.md`, `README.md`'s Layout table |
| Macros | Kielelezo! (design-stage, not implemented) | `docs/design/kielelezo-macros-design.md`, `docs/spec/06-tooling-and-ecosystem.md` |
| Data-Shapes-and-Memory | Seti, Namba_Kuu/Sahihi, Kasha_GC, Faili/Mkondo/Kumbukumbu, ownership | `docs/design/data-shapes-design.md`, `kasha-gc-design.md`, `faili-mkondo-design.md` |
| Wasm-Driver | `driver/wasm` dual-target build | `docs/design/wasm-driver-design.md` |
| Hardware-and-Data | `wazi` + Takwimu/Akili (design-stage, not implemented) | `docs/design/wazi-hardware-design.md`, `takwimu-akili-design.md` |
| Design-Decisions | Curated summary of `docs/spec/08-resolved-decisions.md` | `docs/spec/08-resolved-decisions.md` |
| Roadmap | Phase I/II/III checklist — **update whenever implementation-status.md changes** | `docs/design/implementation-status.md`, `docs/spec/07-execution-and-roadmap.md` |
| Production-Readiness | Mirrors `docs/design/pata-production-readiness.md` | `docs/design/pata-production-readiness.md` |
| Contributing | Dev setup, Git-Flow workflow, testing | `CONTRIBUTING.md` |
| Code-of-Conduct | Mirrors `CODE_OF_CONDUCT.md` | `CODE_OF_CONDUCT.md` |
| Security-Policy | Mirrors `SECURITY.md` | `SECURITY.md` |
| FAQ | Synthesized Q&A, not tied to one source file | none — no docs/ counterpart, skip the sync step for this page |
| Changelog | Summary of `CHANGELOG.md` + `docs/spec/CHANGELOG.md` — **update on every release** | `CHANGELOG.md`, `docs/spec/CHANGELOG.md` |
| _Sidebar | Persistent nav shown on every wiki page — keep its link list in sync with Home's | none (wiki-internal) |

## Standing conventions (apply to every edit)

- **Density over filler.** Every claim traces to a real file/function/flag/command in this repo. No generic marketing sentences. Prefer tables and short code excerpts over prose.
- **Status flags are mandatory.** If what you're documenting is a stub, design-only, or partially implemented, say so explicitly and cite `docs/design/implementation-status.md` — don't present planned work as shipped. Existing pages use a `> **Status: ...**` blockquote near the top for this; match that pattern.
- **Top-of-page sourcing line.** Each page opens with either `Normative source: [...]` (links straight to the file(s) it's derived from) or `Source(s): [...]`. Keep this line accurate when the underlying doc moves or is renamed.
- **Cross-link by wiki page name, not `.md` filename**, e.g. `[CLI Reference](CLI-Reference)` — GitHub wiki links are extension-less.
- **Don't silently resolve doc/code contradictions — and don't stop at noting them.** If the
  source of truth (actual `.rs` source) disagrees with `docs/design/implementation-status.md`, an
  older design doc, or the spec, fix the stale `docs/` file in the same pass (not just a footnote
  on the wiki page) and say so in both places' commit messages. A wiki page that's quietly more
  accurate than the `docs/` file it's derived from is a bug, not a feature — they're supposed to
  say the same thing. Four such discrepancies were found and only *noted* (not yet fixed in
  `docs/`) when the wiki was first built on 2026-09-13 — fix these the next time you touch the
  relevant page or file:
  1. `README.md`'s `sambamba` stdlib functions (`anza_mwendo`/`subiri_mwendo`) are stale — real
     builtins are `tenda`/`subiri_tenda`/`njia`/`fungo`. Fix in `README.md`, reflected already in
     wiki's Concurrency-and-Async.
  2. `docs/howto/00-getting-started.md` says `pata jaribu --list` — real flag is `--orodha`. Fix
     in that howto file, reflected already in wiki's Getting-Started/CLI-Reference.
  3. `docs/SPECIFICATION.md`/`docs/spec/07-execution-and-roadmap.md` call the enum feature
     `Jumla<T>` — the real implemented keyword is `jenum`. Either fix the spec's terminology or
     add the same clarifying note the wiki has (Data-Structures, Roadmap) directly into the spec
     file.
  4. `docs/design/implementation-status.md` says `pata thibitisha` checks only 2 of 6 things —
     current source implements all 6. Fix that doc's checklist entry, reflected already in wiki's
     Formatting-and-Linting.

## Workflow

1. **Clone the wiki fresh** into the scratchpad (never assume a stale local copy is current — someone else may have edited via the GitHub web UI):
   ```bash
   git clone https://github.com/victorKariuki/Asili.wiki.git <scratchpad>/Asili.wiki
   ```
2. **Identify affected pages AND their docs/ counterparts** using the table above. A single code
   change often touches 2+ wiki pages (e.g. a new CLI flag → CLI-Reference, possibly
   Getting-Started or Package-Management, possibly Roadmap if it closes a checklist item) — check
   each affected page's docs/ column too, even if only one page seems obviously relevant.
3. **Re-derive the content from the current source**, don't just patch around the edges — read the
   actual changed file(s) in the main repo (source code, not old doc text) as the source of truth
   for both the wiki page and its docs/ counterpart.
4. **Update the wiki page(s) in the cloned wiki working copy, AND the corresponding docs/ file(s)
   in the main repo working tree, in the same pass.** These are two different git repos with two
   different working trees — you'll be editing files in both
   `<scratchpad>/Asili.wiki/` and the main repo (e.g.
   `/mnt/1264f5d2-2c2b-4f68-8c81-0b146eb0ed58/Projects/Asili/docs/...`). Don't finish one and call
   it done — both sides of every row in the table above should say the same thing when you're
   finished.
5. **Update `_Sidebar.md`** if you added or removed a wiki page (keep it and Home's "Wiki Contents" section identical in link set).
6. **Verify internal wiki links still resolve** — every `](PageName)` must match an actual page's basename:
   ```bash
   cd <scratchpad>/Asili.wiki
   grep -ohE '\]\([A-Za-z][A-Za-z0-9_-]*\)' *.md | sed -E 's/^\]\(//; s/\)$//' | sort -u > /tmp/linked.txt
   ls *.md | sed 's/\.md$//' | sort > /tmp/actual.txt
   comm -23 /tmp/linked.txt /tmp/actual.txt   # should be empty
   ```
7. **Commit and push both sides.** The wiki push is a public-facing push to GitHub — per standing
   house rules, confirm with the user before pushing unless they've already asked for the update
   in this same request. The `docs/` change is a normal commit in the main repo — follow the same
   repo conventions (don't push to a shared branch without the same confirmation) as any other
   change here.
   ```bash
   # wiki side
   cd <scratchpad>/Asili.wiki
   git add -A
   git commit -m "<what changed and why, one line>"
   git push origin master

   # main repo side — same change, matching commit message
   cd /mnt/1264f5d2-2c2b-4f68-8c81-0b146eb0ed58/Projects/Asili
   git add <the specific docs/ file(s) touched>
   git commit -m "docs: <what changed and why, matching the wiki commit>"
   ```

## When NOT to touch the wiki

- Pure refactors with no behavior/API/doc-visible change.
- Internal implementation details not reflected in any page's scope (e.g. renaming a private helper function).
- Work-in-progress branches — update the wiki once a change lands on the branch that ships (typically `main`/`develop`), not speculatively.
