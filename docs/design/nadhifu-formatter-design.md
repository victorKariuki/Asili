# `pata nadhifu` formatter design

The original `canonical_format` (`pata/fmt/src/format.rs`) was a line-by-line
`String::replace`-based transform: `.replace("{", " { ")`, `.replace(",", ", ")`, and so on. This
mangled any string or char literal containing those characters (`"a, {b} c"` became `"a,  { b }
c"`), since it had no concept of "inside a string literal" at all.

## Token-stream, not a full CST/lossless-syntax-tree rewrite

The correct fix for the bug class above is to stop treating source as raw text and start
tokenizing it first — but a full CST (concrete syntax tree preserving every token, including
whitespace/comments, losslessly) was rejected as more than this pass needed. `core/parser`'s AST
nodes carry only a start `line`/`column` today, not an end span, so a CST-based rewrite would
need a larger, cross-cutting AST change (adding end-spans everywhere) before formatting could
even begin. A **token-stream pretty-printer** — re-tokenize, then re-emit tokens with layout
rules driven by token kind and brace/paren nesting depth — sidesteps that entirely: it never
touches the AST, only the lexer's token stream.

The tradeoff: a token-stream printer has no grammar/statement-boundary awareness. It can't tell
"this `<` is a generic bracket" from "this `<` is less-than" by parsing — it has to guess from
local context. Two heuristics carry the weight this would otherwise need real parsing for:

- **Call/index vs. grouping/literal**: `(`/`[` hug their left neighbor (call/index syntax) only
  when that neighbor is an identifier or a closing `)`/`]` — checked against a hardcoded list of
  this language's keywords (`rejesha`, `weka`, `kama`, ...), sourced from every string literal
  `core/parser/src/parse.rs`'s `match_tok` checks against, since the lexer itself has no
  token-kind classification to query. A keyword before `(`/`[` means grouping/array-literal
  instead (`rejesha (x)`, `weka a = [1, 2, 3]`).
- **Generic brackets vs. comparison operators**: a `<` immediately after a capitalized identifier
  opens a *candidate* generic-bracket region; it's confirmed only if a matching `>` (tracking
  nested `<`/`>` depth, for `Kamusi<Neno, Orodha<Namba>>`) is found before any token that could
  only appear in an expression, never a type argument list (`(`, a literal, `;`, `{`, `=`, `->`,
  `?`). This is what keeps `a < b` (comparison) from being misclassified even when `a` happens to
  be capitalized.

Both heuristics are covered by dedicated regression tests (`generic_type_brackets_hug_no_spaces`,
`comparison_operator_not_confused_with_generic_bracket`, `method_call_named_like_a_keyword_hugs_its_parens`,
`grouping_paren_after_keyword_keeps_space`) precisely because they're heuristics, not derivations
from a grammar — a future change to the keyword list or the token shape could silently break one.

## Comment preservation: extend the lexer with an additive, opt-in trivia mode

Comments (`#`/`//`) are lexer-level only — never part of the AST. The original `canonical_format`
implicitly discarded them (working on raw text, it never removed them, but a token-stream printer
built on the *existing* `tokenize()` would, since that function silently skips comments entirely
and produces no trace of them). Since `pata nadhifu` is wired into `pata thibitisha`'s enforcement
path, a formatter that silently deletes developer comments on every run would be actively
harmful, not just incomplete.

**The fix was additive, not a change to `tokenize()`'s behavior**: `core/lexer/src/lib.rs` gained
`tokenize_with_trivia(source) -> (Vec<Token>, Vec<Comment>)`, sharing the same internal
tokenization logic via a `trivia: Option<&mut Vec<Comment>>` parameter, but `tokenize()` itself is
completely unchanged — verified by a test (`tokenize_unaffected_by_trivia_capture`) asserting the
two functions produce byte-identical token streams. This mattered because `tokenize()` has
roughly three dozen call sites across the parser, LSP, lint, and every existing test — changing
its behavior even slightly (e.g. making it return comments as tokens) would have rippled through
all of them. `pata/fmt` is the only consumer of `tokenize_with_trivia`.

Each captured `Comment` records `after_token_index: Option<usize>` — the index of the token it
immediately follows in the *token* stream (`None` if the comment precedes every real token, e.g.
a leading file-header comment). The printer re-emits each comment right after that anchor token,
before moving on to the next real token.

## Blank-line preservation

A gap of 2+ source lines between two adjacent tokens means the author left at least one blank
line there; the printer preserves exactly one (never more, matching the previous
implementation's blank-line-consolidation behavior, and never inventing one where none existed).
This is a deliberate, narrow signal — it doesn't try to reconstruct paragraph/section semantics,
just "was there a visual gap here."

## Idempotency

Token-stream printers are naturally idempotent if deterministic (the same token stream always
produces the same output, and formatted output re-tokenizes to the same token stream) — but this
isn't automatic; it has to hold for every rule above, including the two heuristics. Verified
directly (`test_format_idempotent`, `full_example_is_idempotent_and_clean`, and an end-to-end
smoke test across every real file under `examples/` during implementation — two rounds of
formatting produced zero further changes on all of them).

One subtlety this idempotency check caught: the lexer's char-literal token is a debug-repr string
(`CHAR:A` for `'A'`), and printing that verbatim isn't valid Asili syntax — a second format pass
would re-tokenize `CHAR:A` as three separate tokens (`CHAR`, `:`, `A`), silently corrupting the
program. The fix: `render_lexeme()` converts `CHAR:x` back into proper `'x'` syntax (with correct
re-escaping for `\n`/`\t`/`'`/`\\`) before the printer ever sees it, and does the same escape-
sequence correction for string literals — the lexer stores a string's *decoded* content in its
token (a real newline byte, not the two characters `\` + `n`), so printing it raw would both
corrupt the output and fail the same re-tokenization test.

## Verification

`core/lexer/src/lib.rs`'s own test module (8 tests, including the trivia-capture-doesn't-change-
tokenize test). `pata/fmt/src/format.rs`'s own test module (24 tests): spacing, idempotency,
blank-line consolidation, string/char literal survival, comment preservation (leading-file,
inline, and block-nested), generic-bracket and call/index hugging (including the nested-generics
and comparison-operator-not-confused cases), a keyword-that's-also-a-method-name case, no
trailing whitespace on any line, and the char-literal/string-escape round-trip and idempotency
cases above. End-to-end: every `.as` file under `examples/` was copy-formatted, checked for
idempotency (`--kagua` reports zero files needing changes on a second pass), and every project
with a `pata.toml` was built and run through the real `pata-cli` binary before and after
formatting, confirming byte-identical program output.
