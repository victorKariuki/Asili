# `wazi` (unsafe block) and hardware primitives design

**Status: not started.** No `wazi` keyword exists anywhere in the lexer or parser — confirmed by
grep across `core/lexer/src/*.rs` and `core/parser/src/*.rs`; the only occurrences of the string
`wazi` in the whole repo are in spec prose. Phase IV, per
[docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md). This doc is
forward-looking, and notes one real inconsistency in what's already built.

## What `wazi` is supposed to gate

Per the spec, `wazi` is Asili's unsafe-block equivalent — the explicit escape hatch for manual
memory management and hardware access that the language's default ownership model otherwise
forbids. Three spec sections describe what lives inside it:

- **`01-philosophy-and-edp.md`**: "`wazi` (unsafe) blocks remain the escape hatch for manual
  memory and pointers" — positioned as the *only* sanctioned way to step outside
  ownership/borrowing, paralleling Rust's `unsafe`.
- **`04-type-system.md`**: `Biti8`/`Biti16`/`Biti32`/`Biti64`/`uBiti*` fixed-width integer types
  ("used inside `wazi` blocks or when performance/memory is critical" — so these are usable
  outside `wazi` too, unlike the rest of this list) and `Kiashiria` (raw pointer, "used only in
  `wazi` blocks").
- **`05-standard-library.md`**: hardware-control functions (`vuta_vifaani` — accelerator
  offload, `anwani_ya` — address-of, `tenga_kumbukumbu` — manual heap allocation, `piga_pini` —
  GPIO) and two typed handles, `Pini` (GPIO pin) and `Bafa<T>` (aligned accelerator buffer).

None of `Kiashiria`, `Pini`, `Bafa<T>`, or any of the four hardware functions exist anywhere in
`core/parser/src/ast.rs`'s `ValueType` enum or `core/evaluator/src/value/mod.rs`'s `Value` enum
— confirmed by grep. Unlike `Mfululizo`/`Seti`/`Namba_Kuu`/`Namba_Sahihi` (see
[data-shapes-design.md](data-shapes-design.md)), there isn't even a placeholder TODO comment
naming these as pending `Value` variants. This is earlier-stage than the data-shapes work: no
type-level recognition exists yet at all.

## An existing inconsistency: `syscall` is callable without any `wazi` gate

`core/evaluator/src/builtins/syscall.rs` registers a raw `syscall(nr, a, b, c)` builtin — a
direct analog to the "hardware-level control... inside `wazi` blocks" the spec describes,
conceptually adjacent to (though not literally named in) the spec's hardware-primitives list.
It's a stub today (`FIXME(Phase IV): always returns Anuani(0) — no actual syscall is issued`),
but nothing about its wiring restricts it to a `wazi` context: it's registered like any other
builtin module function, reachable from anywhere via `leta syscall` with no unsafe-block
requirement, no special diagnostic, nothing distinguishing it from a call to `hisabati.jumla`.

This matters for whenever `wazi` is actually implemented: **either `syscall` needs to be
re-gated to require a `wazi` block once one exists** (a breaking change to any code that already
calls it, however unlikely given it currently does nothing), **or `wazi`'s scope needs to be
understood as narrower than "all raw hardware/syscall access"** (i.e., `syscall` was always
meant to be usable without `wazi`, and `wazi` is specifically for the pointer/GPIO/accelerator
surface the standard-library section names). This should be decided explicitly, not left to
whichever gets implemented first.

## Design questions

- **What does `wazi { ... }` actually change, mechanically?** In Rust, `unsafe` doesn't change
  what code executes — it lifts specific compiler-enforced guarantees (no dereferencing raw
  pointers, no calling unsafe fns, etc.) within the block. Does Asili's `wazi` work the same way
  (a lexical scope that relaxes semantic-analyzer checks — e.g. allows `Kiashiria` operations
  that would otherwise be rejected), or is it closer to a capability/permission gate at the
  function-call level (certain builtins simply refuse to be called outside a `wazi` context,
  checked at the call site rather than by relaxing static checks)? These have different
  implementation shapes: the first needs the semantic analyzer to track "are we inside `wazi`"
  as scope state (similar to how it already tracks `loop_depth` for `vunja`/`endelea`
  validity — a real, existing precedent in `core/parser/src/semantic/analyzer.rs`); the second
  is a simpler runtime check inside each gated builtin.
- **`Kiashiria` (raw pointer) representation.** The existing `Anuani` type
  (`core/evaluator/src/value/mod.rs`, `Value::Anuani(u64)` — "raw memory address, used by
  syscall and kiungo") is already a bare-address value type used by exactly the two stubs
  (`syscall`, `kiungo`) that `wazi`-gated code would plausibly use. Is `Kiashiria` meant to be a
  new, distinct type from `Anuani`, or is `Anuani` actually the spec's `Kiashiria` under a
  different name (the same naming-discrepancy pattern flagged for `sambamba`/`tenda` in
  [concurrency-async-design.md](concurrency-async-design.md) — worth checking deliberately
  rather than assuming)? If they're meant to be the same thing, `Anuani` should either be
  renamed or the spec updated to use it, rather than introducing a second overlapping type.
- **`Pini`/`Bafa<T>` are inherently platform-specific.** GPIO pin access has no meaning on a
  desktop/server target — these types only make sense for an embedded target, which per the
  roadmap's own Phase IV framing ("LLVM, embedded, manual memory") doesn't exist yet either
  (no LLVM backend, no embedded-target build configuration anywhere in the repo). This suggests
  `Pini`/`Bafa<T>`/`piga_pini` genuinely can't be implemented meaningfully before an embedded
  compilation target exists — implementing them against this interpreter's current
  desktop/server-only tree-walk evaluator would have nothing real underneath to control.
- **`tenga_kumbukumbu` (manual heap allocation) and the existing allocator-failure model.**
  `07-execution-and-roadmap.md`'s "Embedded / strict mode" section already specifies that
  allocation failure should surface as `Tokeo<Tupu, Kosa>` in strict/embedded configurations —
  `tenga_kumbukumbu`'s return type needs to follow that same already-resolved convention
  (`08-resolved-decisions.md`'s allocation-failure rule) rather than introducing a separate
  error-handling convention for manual allocation specifically.

## Order of work, if this phase starts

1. Resolve the `Anuani`/`Kiashiria` naming question and the `syscall`-gating question — both
   cheap to settle now, expensive to leave ambiguous once real logic exists.
2. Decide the `wazi` mechanism (static relaxation vs. runtime capability check) — this is the
   actual architectural decision; it shapes whether `wazi` needs semantic-analyzer scope
   tracking or is purely a runtime concern.
3. `vuta_vifaani`/`anwani_ya`/`tenga_kumbukumbu` (the three hardware functions that don't need an
   embedded target to make sense — an accelerator, an address-of operator, and manual heap
   allocation are all meaningful on desktop/server) can be implemented once (1) and (2) are
   settled.
4. `Pini`/`Bafa<T>`/`piga_pini` (the GPIO/embedded-specific surface) should wait for an actual
   embedded compilation target to exist — implementing them earlier has nothing real to control.

## Cross-references

- [implementation-status.md](implementation-status.md) — Phase IV is listed not started; the
  `syscall`-without-`wazi`-gate finding here is new since that summary was last written.
- [concurrency-async-design.md](concurrency-async-design.md) — the sibling Phase IV doc with the
  same naming-discrepancy pattern (`sambamba`/`tenda`) that `Anuani`/`Kiashiria` should be
  checked against.
- [data-shapes-design.md](data-shapes-design.md) — the more-advanced sibling doc (type-level
  recognition already exists there; nothing comparable exists yet for this doc's types).
- [docs/spec/01-philosophy-and-edp.md](../spec/01-philosophy-and-edp.md) /
  [docs/spec/04-type-system.md](../spec/04-type-system.md#other-primitives) /
  [docs/spec/05-standard-library.md](../spec/05-standard-library.md#hardware--nguvu-inside-wazi)
  — normative `wazi`/hardware surface this doc is drawn from.
