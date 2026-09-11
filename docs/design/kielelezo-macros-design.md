# Kielelezo! (macros) design

**Status: not started.** No macro syntax (`Kielelezo!`-style invocation or definition) exists
anywhere in the lexer, parser, or AST — confirmed by grep across `core/lexer/src/*.rs`,
`core/parser/src/*.rs`. Per
[docs/spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) and
[docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md), the spec's own
framing is a genuine, checkable precondition, not just a phase label: *"Macro expansion and
hygiene are part of the compiler pipeline; phased after parser and type checker are stable"* and
the dependency note repeats it: *"Require a stable AST and hygiene story; phase after parser and
type checker are stable."*

## Whether the precondition is actually met

"Parser and type checker are stable" is a real, evaluable claim, not just a scheduling
placeholder — worth checking against what this session's other design docs found, since several
of them describe active gaps in exactly those two components:

- The semantic analyzer ("type checker") has multiple open items:
  [phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md)'s
  lifetime-inference strategy is undecided, `SEM120` unconditionally rejects any reference-
  returning function, and `implementation-status.md`'s known-gaps list already names several
  smaller analyzer holes (builtin-method-call fallback silently accepting typos, generic
  constructors not propagating argument types).
- The AST has active TODOs of its own —
  [data-shapes-design.md](data-shapes-design.md)'s `Mfululizo`/`Seti`/`Namba_Kuu`/`Namba_Sahihi`
  are recognized at the type-parsing level but have no `Value` backing, meaning the AST/type
  system is still actively growing new variants for existing spec commitments, let alone stable
  in the "won't need structural changes" sense a macro-hygiene system would want.

None of this is disqualifying on its own — "stable" doesn't have to mean "finished," and a
macro system could reasonably be designed to tolerate some ongoing AST growth. But it's worth
being explicit that the spec's own stated precondition isn't unambiguously met yet, rather than
treating "parser and type checker are stable" as already-satisfied boilerplate.

## What little the spec commits to

`06-tooling-and-ecosystem.md`'s entire treatment: *"Kielelezo! — Code-generation tools to reduce
boilerplate and friction."* That's the complete normative surface — no syntax example, no
distinction between declarative (pattern-substitution, Rust `macro_rules!`-style) and procedural
(arbitrary-code-driven, Rust proc-macro-style) macros, no statement on hygiene semantics beyond
naming "hygiene" as a prerequisite concern. Everything below is open design space, not a gap
between spec and implementation.

## Open design questions

- **Declarative vs. procedural, or both?** A declarative, pattern-substitution macro system
  (think Rust's `macro_rules!`, or a simpler token-pasting scheme) is a much smaller undertaking
  than a procedural system (which needs to expose the AST to user-written expansion code,
  meaning user code that runs *during compilation* — a much larger trust/sandboxing/API-surface
  question, and one this interpreter has no existing story for: there's no "run this Asili code
  at compile time" mechanism anywhere today). Given the stated goal ("reduce boilerplate and
  friction," not "enable compiler plugins"), a declarative-only system is the more conservative
  and more clearly in-scope starting point.
- **Hygiene model.** The spec names hygiene as a concern without specifying an approach.
  Reasonable options, in rough order of implementation cost: no hygiene at all (simplest,
  historically error-prone — accidental variable capture between macro-generated and
  call-site code); syntactic hygiene via renamed/gensym'd identifiers for macro-internal
  bindings (Rust's actual approach, moderate complexity); full scope-tracking hygiene (most
  correct, most implementation work, arguably overkill for a "reduce boilerplate" feature rather
  than a "safe, composable metaprogramming" feature).
- **Invocation syntax.** `Kielelezo!` (with the `!`) mirrors Rust's macro-invocation convention
  directly — worth confirming this is intentional signal-following (this language's spec
  elsewhere explicitly borrows recognizable syntax where it doesn't conflict with the Swahili-
  keyword design) rather than a placeholder name, before committing lexer/parser grammar to it.
- **Where expansion happens in the pipeline.** The spec says "part of the compiler pipeline" —
  concretely, does expansion happen as a pass between parsing and semantic analysis (macros
  produce AST nodes that then get type-checked normally, the simplest integration point given
  the existing `tokenize → parse_tokens → semantic_check → evaluate` pipeline structure this
  repo already has), or does it need to interleave with semantic analysis itself (for macros that
  need type information to decide their expansion — a much harder problem, akin to what
  `#[sharti]`'s target-based filtering does at a much simpler level, see
  [sharti-design.md](sharti-design.md) for how that existing filter pass is structured as a
  precedent for "a pass that runs between parsing and semantic analysis and prunes/transforms
  the `Module`")? Given the "reduce boilerplate" framing, a syntactic (pre-type-check) expansion
  model is almost certainly sufficient and should be the default assumption unless a concrete use
  case demands otherwise.

## Order of work, if this phase starts

1. Explicitly re-confirm (or consciously waive) the "parser and type checker are stable"
   precondition — the honest current state, per above, is "actively growing," not "stable."
2. Decide declarative-only vs. declarative+procedural (recommend declarative-only first, given
   the stated goal and the lack of any compile-time-code-execution story today).
3. Decide the hygiene model (recommend starting with syntactic/gensym-style hygiene — the
   Rust-precedent middle ground — rather than either extreme).
4. Design expansion as a pipeline pass between parsing and semantic analysis (mirroring
   `#[sharti]`'s existing filter-pass structure as an integration-point precedent), producing
   ordinary AST nodes that the existing semantic analyzer and evaluator need no macro-specific
   changes to handle.

## Cross-references

- [implementation-status.md](implementation-status.md) — no existing checklist entry for this
  subsystem (absent from the phase checklist the same way it's absent from the roadmap's
  feature–phase map for anything before Phase IV's brief mention).
- [sharti-design.md](sharti-design.md) — the closest existing precedent for "a pipeline pass that
  runs between parsing and semantic analysis and transforms/prunes a `Module`."
- [phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md) /
  [data-shapes-design.md](data-shapes-design.md) — the concrete evidence that "parser and type
  checker are stable" isn't fully true yet.
- [docs/spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md#kielelezo-macros) /
  [docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md#dependency-notes)
  — normative (minimal) macro description and the stability precondition.
