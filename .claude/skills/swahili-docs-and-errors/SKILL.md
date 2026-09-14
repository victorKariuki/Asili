---
name: swahili-docs-and-errors
description: Write or review user-facing Asili text — compiler/runtime/CLI error messages, diagnostic codes' messages, REPL help topics (docs/repl/sw/), and any user-facing doc meant to read in Swahili — in grammatically correct, idiomatic Swahili matching the project's existing register. Use whenever adding a new Diagnostic/error message in core/ or pata/, adding or editing a docs/repl/sw/*.md page, or the user asks for Swahili copy, error text, or a grammar/translation check.
---

# Writing Swahili user-facing text for Asili

Asili's user-facing surface is Swahili-first: source keywords, CLI subcommands and flags,
compiler/runtime diagnostics, and one whole tree of docs (`docs/repl/sw/`) are Swahili, not
English translated at render time. Getting the grammar right matters here the way getting error
wording right matters in any language — sloppy Swahili reads to a native speaker the way "Error:
file not find" reads to an English speaker.

## Where this applies

- **Diagnostic messages** — `Diagnostic::new("SEMxxx"/"LINTxxx"/"SHAxxx", "...")` calls in
  `core/parser/src/semantic/analyzer.rs`, `pata/lint/src/rules/*.rs`, and anywhere else a
  `Diagnostic` is constructed with a message string.
- **CLI output strings** — anything a `pata` subcommand prints (`pata/cli/src/commands/*.rs`):
  success/failure messages, prompts, `--husaidia` help text.
- **REPL help docs** — `docs/repl/sw/*.md` (the Swahili half of the bilingual REPL topic docs,
  shown via `?mada` after `?lugha sw`).
- **Runtime panic/error text** — anything surfaced to a program author via `paparika`, `makosa`,
  or a builtin's `Err(...)` message.

**Not in scope:** `docs/language/`, `docs/spec/`, `docs/howto/`, `docs/design/`, this repo's own
`CLAUDE.md`/`CONTRIBUTING.md`, commit messages, and `docs/repl/en/` — those are contributor/spec
docs and stay in English by existing convention. Don't Swahili-ify them.

## Grammar and register rules

Derived from the actual existing corpus (`docs/repl/sw/*.md`, and real `Diagnostic::new(...)`
message strings in `core/parser/src/semantic/analyzer.rs`) — match this register, don't invent a
new one:

1. **Lowercase, terse, verb-first or subject-first sentences** — not Title Case, not English
   sentence structure translated word-for-word. Real examples from the codebase:
   - `"{op_sym} inahitaji Namba pande zote mbili"` (SEM014)
   - `"umbo '{}' linahitaji uga '{}'"` (SEM097)
   - `"sharti thamani haiwezi kuwa tupu"`
   - `"jina la mada si sahihi"`
2. **Correct noun-class agreement on verbs.** Swahili verb prefixes agree with the subject's noun
   class — this is the single most common way machine-translated or non-native Swahili breaks.
   - `umbo` (class 5/6, *ji-/ma-*) → `linahitaji` (li- prefix), not `inahitaji`.
   - `Namba`, `Neno`, most borrowed/abstract nouns → class 9/10 (*i-/zi-*) → `inahitaji`,
     `haiwezi`, `imeshindwa`.
   - `kazi` (class 9/10) → `haiwezi` as in `"kazi kuu haiwezi kuwa ndani ya shughuli"`.
   - When in doubt, check how the existing corpus refers to the same noun elsewhere
     (`grep -rn "<noun>" core/parser/src/semantic/analyzer.rs docs/repl/sw/`) rather than guessing
     — consistency with existing usage beats grammatical purity if the corpus has already made a
     (defensible) choice.
3. **Verb tense/aspect matches the failure's nature**, not a flat translation of English tense:
   - A completed action that failed: *im-* perfect — `imeshindwa kupakia workspace`.
   - A standing requirement/constraint: *-na-* present habitual — `inahitaji`, `haiwezi`.
   - Don't default to literal present-continuous English "-ing" → Swahili *-na-* mapping when the
     real semantics are perfect/completive.
4. **Use the project's own vocabulary, not a fresh translation.** Check
   `docs/spec/08-resolved-decisions.md` §9.2–9.3 and the keyword set in `docs/language/` /
   `docs/repl/sw/` before introducing a new Swahili term for a concept that already has one (e.g.
   always `kosa`/`makosa` for error, never a synonym like `hitilafu` unless the existing corpus
   already uses it in that exact spot — `hitilafu` and `kosa` currently coexist for slightly
   different registers in the corpus, e.g. `"hitilafu ya kusoma entry"` vs. `"makosa"` as the
   module name — match whichever the immediate context already uses).
5. **No English loanwords where a real Swahili term exists and is already used elsewhere in the
   corpus.** Technical terms without an established Swahili equivalent in this project (e.g.
   proper nouns, `TOML`, `WASI`) stay as-is rather than being forced into an awkward coinage — see
   `"hitilafu ya kubadili manifest kuwa TOML"` for the existing pattern of keeping a loanword
   untranslated but grammatically integrated into the sentence.
6. **Structural parity with English docs where both exist.** `docs/repl/sw/*.md` and
   `docs/repl/en/*.md` must have matching `##` section structure (same topics, same order,
   verified: currently in parity) — a Swahili REPL doc page is a real translation of its English
   counterpart's content, not independently authored. When editing one, edit both, and keep
   section headers 1:1.

## Workflow

1. **Before writing new Swahili text**, grep the existing corpus for how the same noun/verb/error
   concept is already phrased:
   ```bash
   grep -rn "<keyword>" core/parser/src/semantic/analyzer.rs pata/lint/src/rules/ docs/repl/sw/
   ```
2. **Draft the message**, applying the noun-class agreement rules above.
3. **If it's a REPL doc (`docs/repl/sw/*.md`)**, check the matching `docs/repl/en/*.md` file's
   section headers and update both files to keep them in parity.
4. **If it's a new Diagnostic**, follow the existing `SEMxxx`/`LINTxxx` numbering convention in
   that file (next unused number in sequence) and match the terse, lowercase phrasing style shown
   above — no trailing period, no capital first letter, matching every existing message in that
   file.
5. **Self-check before finishing**: read the sentence aloud (mentally) checking subject↔verb
   noun-class agreement, and confirm no message reads as an English sentence with Swahili words
   substituted in — restructure if it does. When genuinely unsure of a grammar point the corpus
   doesn't already resolve, say so rather than guessing confidently.
