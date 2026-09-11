# Data shapes design: `Mfululizo`, `Seti`, `Namba_Kuu`, `Namba_Sahihi`

**Status: not started at the runtime level; further along at the type level than it might
appear.** Per [implementation-status.md](implementation-status.md#phase-i--catalyst-core-interpreter),
these four types are the concrete remainder of Phase I's scope, deferred to Phase III. This doc
exists to record what already exists to build on (more than the checklist bullet alone
communicates) and the open design questions for each.

## What already exists: type-level recognition, no runtime backing

`core/parser/src/semantic/types.rs`'s `parse_value_type` already recognizes all four type names
and produces real `ValueType` variants:

```rust
// types.rs
if s.starts_with("Mfululizo<") && s.ends_with('>') { return ValueType::Mfululizo(...); }
if s.starts_with("Seti<") && s.ends_with('>') { return ValueType::Seti(...); }
if s == "Namba_Kuu" { return ValueType::NambaKuu; }
if s == "Namba_Sahihi" { return ValueType::NambaSahihi; }
```

`core/parser/src/ast.rs` has matching `ValueType::Mfululizo(Box<ValueType>)`,
`ValueType::Seti(Box<ValueType>)` variants (`Namba_Kuu`/`Namba_Sahihi` appear to be simple unit
variants per the type name matching above — confirm exact `ValueType` shape before implementing,
this doc is describing what the parser *recognizes*, not asserting the full enum shape) and a
`Display` impl that round-trips them back to `Mfululizo<T>`/`Seti<T>` syntax.

**None of this has a runtime `Value` counterpart.** `core/evaluator/src/value/mod.rs` carries an
explicit TODO comment naming exactly what's missing:

```rust
// TODO(Phase II): Missing value variants from the spec:
//   - Seti(HashSet<MapKey>) — ordered set type (Seti<T>)
//   - Mfululizo(&[Value]) — slice/view into an Orodha without cloning (requires lifetime or Rc)
//   - NambaKuu(BigInt) — arbitrary-precision integer (Namba_Kuu); needs the `num-bigint` crate
```

(Labeled `Phase II` in that comment — since superseded; `implementation-status.md` places this
work in Phase III.) So today: `weka s: Seti<Namba> = ...` parses and produces a `ValueType::Seti`
for the analyzer to reason about, but there is no expression that could ever construct a
`Value::Seti` — no constructor builtin, no literal syntax, nothing for the evaluator to actually
produce or operate on. The type system is ahead of the runtime here, not the reverse.

## `Seti<T>` (Set)

Least architecturally interesting of the four — the note above already specifies the shape
(`HashSet<MapKey>`, reusing the existing `MapKey` type `Kamusi<K,V>` already uses for hashable
keys). Open questions, all narrow:

- Constructor/literal syntax: a builtin function (`seti(...)`, mirroring `orodha(...)`/
  `kamusi(...)`) is the obvious choice, consistent with how every other collection type is
  constructed in this language (no dedicated literal syntax beyond `[]`/`{}` for Orodha/Kamusi
  already exists, and those are arguably special-cased enough already).
- Method surface: `.ongeza(x)`/`.ina(x)` (contains)/`.ondoa(x)`/`.urefu()`, mirroring `Orodha`'s
  and `Kamusi`'s existing method-naming conventions.
- Iteration order: `HashSet` gives none; if `linganisha`/`kwa...katika` iteration needs
  deterministic order for debugging/testing, an `IndexSet`-style crate might be preferable to
  std's `HashSet` — a real but small decision, not a blocker.

## `Mfululizo<T>` (Slice)

The `TODO` comment's own parenthetical — "requires lifetime or Rc" — is the real blocker, and
it's the same blocker as the Phase III borrow-checker work (see
[phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md)): a
slice is fundamentally a *borrowed view* into an `Orodha`'s backing storage. Building
`Value::Mfululizo` before lifetimes exist means either:

- **Fake it with `Rc`** — `Mfululizo(Rc<[Value]>)` or similar, giving slice-like read access
  without true borrowing, at the cost of an extra allocation/indirection and no actual
  aliasing-safety guarantee beyond what `Rc` itself provides (shared ownership, not a borrow).
  This sidesteps the lifetime blocker entirely but produces something that behaves more like "a
  cheap-clone read-only view," which may or may not match what the spec means by `Mfululizo`
  (`04-type-system.md` just says "View into Orodha or buffer," which is compatible with either
  reading).
- **Wait for real references** — implement `Mfululizo<T>` only once `Rejeo<T>`'s runtime backing
  exists (Phase III's other blocker), so a slice is genuinely `&[Value]`-shaped with real
  lifetime tracking.

This is the one item in this doc that's genuinely blocked on another design doc's open question
(the lifetime-inference strategy), not just unstarted.

## `Namba_Kuu` (BigInt) / `Namba_Sahihi` (BigDecimal)

Two separate types, same shape of work: wrap an external arbitrary-precision crate
(`num-bigint`'s `BigInt` is explicitly named in the evaluator's own TODO comment;
`num-bigint`'s `BigDecimal`-equivalent or a separate `bigdecimal` crate would cover
`Namba_Sahihi`). Design questions:

- **Arithmetic operator overloading.** Today's `BinaryOp::Add`/`Sub`/etc. dispatch through
  `binary_f64` (see `core/evaluator/src/eval/expr.rs`), which unconditionally coerces both
  operands to `f64` via `value::as_f64`. `Namba_Kuu`/`Namba_Sahihi` values can't round-trip
  through `f64` without losing the entire point of arbitrary precision — the binary-operator
  dispatch needs a new arm per operator that recognizes `Value::NambaKuu`/`Value::NambaSahihi`
  operands *before* falling through to the `f64` path, not just a `value::as_f64` coercion added
  to the existing helper.
- **Casting.** `04-type-system.md`'s casting rules (`kama`) would need explicit
  `Namba`↔`Namba_Kuu`↔`Namba_Sahihi` conversion semantics — is `42 kama Namba_Kuu` infallible
  (always succeeds, `Namba` is a strict subset)? Is `namba_kuu_val kama Namba` fallible (a
  `Chaguo`/`Tokeo`, since a huge `Namba_Kuu` can't fit in `f64` precision)? The existing `Biti8`-
  style fallible-cast pattern (`Chaguo<T>` return, `core/evaluator/src/eval/expr.rs`'s Cast
  handling) is the closest precedent to reuse for the narrowing direction.
- **Literal syntax.** Does a `Namba_Kuu` literal need its own suffix (e.g. Rust's `123u128`-style
  suffix), or is it always constructed via an explicit cast/constructor from a `Neno` (parsing a
  decimal-digit string, the way genuinely-arbitrary-precision values are usually entered)? The
  latter avoids any lexer/grammar changes at all — a real simplification worth taking if nothing
  else forces literal syntax.

None of these three questions are hard blockers the way `Mfululizo`'s lifetime dependency is —
they're implementation-shaped decisions that can be made independently, in either order, whenever
this work is picked up.

## Order of work, if this phase starts

1. `Seti<T>` first — no blockers, most similar to existing `Orodha`/`Kamusi` patterns.
2. `Namba_Kuu`/`Namba_Sahihi` — independent of `Seti`, needs the operator-dispatch and
   casting-semantics decisions above made first, but no cross-doc dependency.
3. `Mfululizo<T>` last — genuinely blocked on the lifetime-inference decision in
   [phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md),
   unless the `Rc`-based "fake it" approach is chosen deliberately as a stopgap.

## Cross-references

- [implementation-status.md](implementation-status.md#phase-i--catalyst-core-interpreter) —
  phase checklist entry this doc expands.
- [phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md) —
  `Mfululizo<T>`'s real blocker (lifetime-inference strategy).
- [docs/spec/04-type-system.md](../spec/04-type-system.md#data-shapes-additional) — normative
  type table this doc's four types are drawn from.
