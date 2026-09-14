# Phase III design: self-hosting and the runtime borrow checker

**Status: lifetime-inference strategy decided (see below); no implementation started.** This doc
is forward-looking — it exists to capture the real blockers and existing groundwork so a future
implementer doesn't have to re-derive them, per
[implementation-status.md](implementation-status.md#phase-iii--resolution-self-hosting-borrow-checker--not-started),
whose findings this doc expands on. Unlike the design docs for already-built subsystems
([mwalimu-design.md](mwalimu-design.md) etc.), there is no working code to describe here — only
what exists to build on, what's missing, and (now) the decision that was blocking further work.

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

## The decision: region-based inference with a narrow annotation surface

The roadmap doc itself (per `implementation-status.md`'s summary) already warns not to start the
runtime borrow checker before deciding the lifetime-inference strategy. This was framed as a
three-way open choice (full explicit lifetimes / fully inferred with no annotation surface /
something in between), but re-reading `docs/spec/04-type-system.md`'s existing normative text
(line 132, already resolved, not something this doc gets to relitigate) narrows it to one
option: **"Lifetimes are inferred by default; explicit lifetime annotations are only required
when references cross complex structural boundaries (e.g. stored in `umbo` fields or returned
from functions with non-obvious relationships)."** That sentence rules out both of the other two
candidates directly — full explicit lifetimes contradicts "inferred by default"; fully inferred
with zero annotation surface contradicts "annotations are only required when..." (which asserts
they exist and are sometimes mandatory). So the decision was already made at the spec level; what
was missing was making it concrete enough to implement against. That's what this section does.

**Scope of what's inferred (no annotation, the common case):**
- Any borrow (`azima`/`azima_tenda`) whose lifetime is bounded by a single function body — a
  reference created and used entirely within one `kazi`, including passed into nested block
  scopes, loops, and calls to other functions *as long as the callee doesn't store it past the
  call* (ordinary "borrow a value, use it, done" code, which is the overwhelming majority of
  real borrow usage). This is inferred purely from the existing scope-tree structure the
  semantic analyzer's `Binding` tracking (`created_at`/`moved_at`/`borrowed_at`/`dropped_at`)
  already has — no new syntax, no new AST node, for this case.

**Scope of what requires an explicit annotation (the spec's "complex structural boundaries"):**
1. **A `Rejeo<T>`/`Rejeo_Tenda<T>` stored as an `umbo` field.** A struct holding a reference
   necessarily outlives (or is bounded by) whatever it borrowed from — the relationship isn't
   locally inferable from the struct definition alone, it depends on every call site that
   constructs an instance.
2. **A function that returns `Rejeo<T>`/`Rejeo_Tenda<T>` where the returned reference's lifetime
   isn't syntactically obvious from a single parameter** (i.e., not simply "returns a borrow of
   its only reference parameter," which the region inference can resolve on its own — mirroring
   Rust's own successful "lifetime elision" precedent for the single-input-reference case).

Both cases require exactly one thing: a way to say "this reference's lifetime is tied to *that*
named region." Proposed concrete syntax, modeled on Rust's `'a` but consistent with Asili's
keyword style (a leading tick reads oddly against Swahili keywords, so use a named parameter
introduced by `muda`, matching the `Muda_wa_Kuishi` spec name directly rather than inventing a
new short-hand token):

```asili
umbo Kiashiria_kwa<muda M> {
    thamani: Rejeo<Namba, M>
}

kazi kubwa_kuliko<muda M>(a: Rejeo<Namba, M>, b: Rejeo<Namba, M>) -> Rejeo<Namba, M> {
    ikiwa (jaribu a) > (jaribu b) { rejesha a } vinginevyo { rejesha b }
}
```

`Rejeo<T, M>`/`Rejeo_Tenda<T, M>` gain an optional second type parameter carrying the region name
`M` — omitted (`Rejeo<T>`, today's syntax) when a reference is function-local and fully inferred;
required when it crosses one of the two boundary cases above. `M` is declared the same way a
generic type parameter already is (`kazi f<T>(...)`), via a new `muda` keyword marking it as a
lifetime parameter rather than a type parameter, so the parser can tell the two apart without new
punctuation.

**What this determines for implementation** (not scheduled here, but now unambiguous when it is):
- `Muda_wa_Kuishi` becomes `ValueType::Rejeo(Box<ValueType>, bool, Option<String>)` — extending
  the existing `Rejeo(Box<ValueType>, bool)` (target type, mutability) with an optional named
  region, rather than a wholesale new type. `None` is the inferred/local case; `Some(name)` is
  the annotated case.
- `SEM120`'s "lifetime inference not yet implemented" blanket rejection of reference-returning
  functions gets replaced with: accept if the returned reference's region is inferable (single
  reference parameter, elision-style) or explicitly annotated; reject only the genuinely
  ambiguous case (multiple reference parameters, no annotation, no way to know which one the
  return value is tied to) — which is a **narrower** rejection than today's blanket one.
- `core/evaluator/src/eval/expr.rs:217`'s `BorrowImm`/`BorrowMut` TODO (today: clone, not a real
  reference) is the actual runtime-semantics work this decision unblocks, but implementing it is
  a separate, larger task from making the decision — not undertaken in this pass.

## What self-hosting additionally needs, beyond the borrow checker

Even with lifetimes solved and per-character string indexing added, self-hosting requires writing
a lexer, parser, and (at minimum) a semantic analyzer in Asili itself — none of which currently
exist as example programs, meaning the first real self-hosting attempt would also be the first
large-scale stress test of the language's own ergonomics for writing a compiler (recursive
descent parsing, tree constructions via `umbo`/`jenum`, deep pattern matching via `linganisha`).
No design decision is needed for this today — it's a large implementation effort gated entirely
on the two blockers above, not an open design question of its own.

## `Mfululizo<T>`'s stopgap: not taken

`docs/design/data-shapes-design.md` flags `Mfululizo<T>` (a borrowed slice/view into an
`Orodha`) as blocked on this exact decision, with a fallback option: an `Rc<[Value]>`-backed
"fake it" stopgap that gets *a* `Mfululizo` shipped without waiting for real reference
semantics. **Not taken, deliberately.** Now that the lifetime-inference strategy above is
decided, `Mfululizo<T>` is the concrete first user of the `Rejeo`-with-optional-region-parameter
shape once `eval/expr.rs:217`'s real-reference-semantics work lands (a `Mfululizo<T>` *is*,
semantically, a `Rejeo<Orodha<T>, M>`-shaped view, not an unrelated type) — building it on `Rc`
sharing now would mean rebuilding it again once real borrows exist, rather than sharing that
work. `Mfululizo<T>` stays deferred, tracked in `docs/design/data-shapes-design.md`, until the
runtime-reference-semantics implementation (order-of-work item 3 below) actually lands — not
implemented as part of this decision pass.

## Order of work, if this phase starts

1. ~~Decide the lifetime-inference strategy~~ — **done, see above.**
2. Add per-character string indexing (narrow, contained — a new `Value::Neno` method-dispatch
   arm, `.herufi_kwa(i) -> Chaguo<Herufi>`, grapheme-indexed to match `.urefu()`'s existing
   grapheme-count convention rather than switching to codepoints for this one method).
3. Add the `muda`-parameter/region-name syntax to the parser and `ValueType::Rejeo`'s new
   `Option<String>` field; wire `Rejeo`/`Rejeo_Tenda` to carry real reference semantics in the
   evaluator (replacing the `BorrowImm`/`BorrowMut`-clones-a-copy behavior at `eval/expr.rs:217`).
   `Mfululizo<T>` becomes buildable once this lands (see above).
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
