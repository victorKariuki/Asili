# Phase III design: self-hosting and the runtime borrow checker

**Status: not started, not yet implementable.** This doc is forward-looking — it exists to
capture the real blockers and existing groundwork so a future implementer doesn't have to
re-derive them, per [implementation-status.md](implementation-status.md#phase-iii--resolution-self-hosting-borrow-checker--not-started),
whose findings this doc expands on. Unlike the design docs for already-built subsystems
([mwalimu-design.md](mwalimu-design.md) etc.), there is no working code to describe here — only
what exists to build on, what's missing, and what decisions have to be made first.

Two distinct goals are bundled into "Phase III" by the roadmap
([docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md)): self-hosting (a
compiler for Asili, written in Asili) and a runtime-enforced borrow checker (moves/borrows that
actually affect program behavior, not just static diagnostics). They're bundled because
self-hosting a compiler is exactly the kind of program that most needs real move/borrow
enforcement to be memory-safe without a GC — but they're separable pieces of work with different
blockers.

## Blocker 1: string manipulation, corrected from the earlier record

`implementation-status.md`'s Phase III entry previously stated "there is no string-manipulation
stdlib at all — no split/substring/length/char-index/replace builtins anywhere." **That claim was
wrong in the specifics** — re-checked directly against `core/evaluator/src/eval/expr.rs`'s
`Value::Neno` method-dispatch arms:

| Claimed missing | Actually exists |
|---|---|
| length | `.urefu()` (grapheme count), `.biti_ngapi()` (byte count) |
| substring | `.kata(start, end)` (byte-range slice) |
| split | `.gawanya(sep)` |
| replace | `.badilisha(kwa, na)` |
| find | `.tafuta(sub)` → `Chaguo<Namba>` (byte index) |

**What's genuinely still missing: per-character indexing.** There is no `.herufi_kwa(i)` (char-
at-index) or equivalent — the only way to inspect string contents positionally is `.kata()`'s
byte-range slicing, which (per the `docs/language/08-njia-za-aina.md` finding from an earlier
verification pass) can cut a multi-byte UTF-8 character in half if the boundary is chosen wrong.
A tokenizer written in Asili needs to ask "what is the character at position N" (to classify it
as whitespace/digit/identifier-char/etc.) far more than it needs substring slicing — and that
operation doesn't exist today in a form that's both correct (grapheme/codepoint-aware) and
efficient (not "slice out a 1-byte-or-more range and hope it's a full character").

This needs to be re-scoped before Phase III starts: the missing piece is narrower than "a string
stdlib," but it's a real gap, not a solved problem as the corrected list above might otherwise
suggest.

## Blocker 2: the borrow checker has semantic-analysis-only enforcement

`core/parser/src/semantic/analyzer.rs`'s `Binding` struct already tracks `moved: bool`,
`mut_borrowed: bool`, `imm_borrows: usize` per binding, with real diagnostics (SEM040–044) for
double-move, use-after-move, and immutable/mutable borrow conflicts — confirmed working, and
tested by earlier sessions' example fixes (`azima`/`azima_tenda` conflict detection). This is a
real, working move-checker **at compile time**.

What doesn't exist:

1. **No runtime backing.** `core/evaluator/src/eval/expr.rs:217` has an explicit
   `TODO(Phase III): BorrowImm/BorrowMut should produce Rejeo/Rejeo_Tenda values` — today,
   `azima x`/`azima_tenda x` evaluate to a full clone of `x`'s value, not a reference. The
   semantic analyzer enforces move/borrow *rules*, but the evaluator doesn't actually alias
   anything — two "borrows" of the same variable are, at runtime, just two independent copies.
   This means the borrow checker currently only prevents code from *compiling* in ways that
   would be unsafe if references were real; it provides no actual memory-sharing behavior.
2. **Lifetimes don't exist at all.** `core/parser/src/semantic/analyzer.rs` (`SEM120`) flatly
   rejects any function that returns a reference, with the message "lifetime inference not yet
   implemented." There is no `Muda_wa_Kuishi` (lifetime) type anywhere in the AST — not a stub
   variant, not a parsed-and-ignored placeholder, nothing. `docs/spec/04-type-system.md`'s
   `Rejeo`/`Rejeo_Tenda`/`Muda_wa_Kuishi`/`Kiashiria`/`Gundi` table names the intended surface,
   but none of it exists at the type-system level beyond `Rejeo`/`Rejeo_Tenda` being mentioned
   as what `azima`/`azima_tenda` "desugar to" in prose — no actual `ValueType::Rejeo` variant
   carrying lifetime information exists to check against.

## The decision that has to happen before either blocker is worked

The roadmap doc itself (per `implementation-status.md`'s summary) already warns not to start the
runtime borrow checker before deciding the lifetime-inference strategy. That decision doesn't
exist yet and isn't merely an implementation detail — it changes what's representable in the
type system:

- **Full explicit lifetimes** (Rust-style `'a` annotations) — most expressive, most complexity
  for users; contradicts the spec's stated default of "lifetimes are inferred by default"
  (`04-type-system.md`).
- **Fully inferred, no annotation surface at all** — simplest for users, but inference algorithms
  that handle all the cases Rust needs explicit annotations for are a substantial undertaking,
  and the spec's own hedge ("explicit lifetime annotations are only required when references
  cross complex structural boundaries") implies *some* annotation surface will exist — which one
  is undecided.
- **Region-based/scoped inference with a narrower annotation surface** (something between the
  two) — plausible middle ground, but "narrower" needs to be specified concretely (which cases
  need annotation, what the annotation syntax is) before any AST/analyzer work starts.

Whichever is chosen determines: whether `Muda_wa_Kuishi` needs to exist in the AST at all (only
if some annotation surface exists), what `SEM120`'s eventual replacement check looks like, and
whether `Rejeo<T>`/`Rejeo_Tenda<T>` need a lifetime parameter or can stay lifetime-erased with a
simpler (but less expressive) escape-analysis-style check.

## What self-hosting additionally needs, beyond the borrow checker

Even with lifetimes solved and per-character string indexing added, self-hosting requires writing
a lexer, parser, and (at minimum) a semantic analyzer in Asili itself — none of which currently
exist as example programs, meaning the first real self-hosting attempt would also be the first
large-scale stress test of the language's own ergonomics for writing a compiler (recursive
descent parsing, tree constructions via `umbo`/`jenum`, deep pattern matching via `linganisha`).
No design decision is needed for this today — it's a large implementation effort gated entirely
on the two blockers above, not an open design question of its own.

## Order of work, if this phase starts

1. Decide the lifetime-inference strategy (the real blocker — nothing else in this doc can start
   until this is settled).
2. Add per-character string indexing (narrow, contained — a new `Value::Neno` method-dispatch
   arm, likely `.herufi_kwa(i) -> Chaguo<Herufi>`, grapheme- or codepoint-indexed per whichever
   the decision favors).
3. Add `Muda_wa_Kuishi` to the AST/type system if the chosen strategy needs it; wire
   `Rejeo`/`Rejeo_Tenda` to carry real reference semantics in the evaluator (replacing the
   `BorrowImm`/`BorrowMut`-clones-a-copy behavior at `eval/expr.rs:217`).
4. Only then attempt a self-hosted lexer as the first real stress test.

## Cross-references

- [implementation-status.md](implementation-status.md#phase-iii--resolution-self-hosting-borrow-checker--not-started)
  — phase checklist entry this doc expands and corrects (the string-stdlib claim).
- [docs/spec/04-type-system.md](../spec/04-type-system.md#memory-ownership-and-references) —
  normative `Rejeo`/`Rejeo_Tenda`/`Muda_wa_Kuishi`/ownership rules this doc's blockers block.
- [docs/spec/08-resolved-decisions.md](../spec/08-resolved-decisions.md#910-ownership-and-borrowing)
  — the ownership/borrowing rules already resolved and already enforced at compile time.
- [docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md) — phase/feature
  map this doc's scope is drawn from.
