# Data shapes design: `Mfululizo`, `Seti`, `Namba_Kuu`, `Namba_Sahihi`

**Status: `Seti<T>`, `Namba_Kuu`, `Namba_Sahihi` implemented; `Mfululizo` not started at the
runtime level, blocked on the borrow-checker lifetime decision.** Per
[implementation-status.md](implementation-status.md#phase-i--catalyst-core-interpreter), these
four types are the concrete remainder of Phase I's scope, deferred to Phase III. This doc
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

## `Seti<T>` (Set) — implemented

`Value::Seti(HashSet<MapKey>)` (`core/evaluator/src/value/mod.rs`), reusing the same `MapKey`
type `Kamusi<K,V>` already uses for hashable keys. Every open question above was resolved as
predicted:

- Constructors: `seti(v1, v2, ...)` (variadic — see below) and `seti_tupu()`
  (`core/evaluator/src/builtins/seti.rs`), mirroring `orodha(...)`/`kamusi_tupu()`.
- Methods (`core/evaluator/src/eval/expr.rs`, hardcoded dispatch arms like every other builtin
  collection): `.ongeza(v) -> Tupu`, `.ondoa(v) -> Ukweli` (was a member removed?),
  `.ina(v) -> Ukweli` (contains), `.urefu() -> Namba`, `.clona()`, `.orodha() -> Orodha<T>`
  (materialize into a list, since `kwa...katika`-style iteration has no direct Seti support).
  Mutation (`.ongeza`/`.ondoa`) follows `Kamusi.ingiza`'s existing pattern: clone-modify-then-
  `rt.env.set()` back into the receiver's binding, not true interior mutability.
- Iteration order: shipped with std `HashSet` (unspecified order) as planned. No `IndexSet`
  dependency taken — revisit only if deterministic iteration becomes a real, demonstrated need.
- **One thing not predicted**: `seti`'s `FnContract` (`core/parser/src/builtins.rs`) declares
  `params: vec![]` (matching `orodha`'s own contract shape), but the analyzer's stdlib-call
  arity check (`SEM046`) rejects any call whose arg count doesn't match `params.len()` —
  `orodha`'s variadic-ness is a hardcoded `name == "orodha"` special case in
  `core/parser/src/semantic/analyzer.rs`, not a general "this FnContract is variadic" flag.
  `seti` needed the same hardcoded exception added (`name == "orodha" || name == "seti"`) to
  compile at all. Worth knowing if a future variadic stdlib function needs the same treatment —
  the special-case list, not a systematic mechanism, is exactly where to look.

Always in scope via `msingi` (no `leta seti` gate), matching `Kumbukumbu<T>`'s decision, not
`Kasha_GC<T>`'s opt-in one — the spec's Data Shapes table doesn't flag `Seti` as opt-in.

See `core/evaluator/tests/seti.rs` for test coverage, `examples/seti/` for an end-to-end example.

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

## `Namba_Kuu` (BigInt) / `Namba_Sahihi` (BigDecimal) — implemented

`Value::NambaKuu(num_bigint::BigInt)` / `Value::NambaSahihi(bigdecimal::BigDecimal)`
(`core/evaluator/src/value/mod.rs`). Every open question above resolved as predicted:

- **Arithmetic operator overloading**: `core/evaluator/src/value/numeric.rs`'s
  `big_numeric_binary_op` is checked before `BinaryOp::Add`/etc. fall through to `binary_f64`
  (`core/evaluator/src/eval/expr.rs`). Widening rule: a plain `Namba` operand widens infallibly
  to match whichever big type the other side is; if both sides are `Namba_Kuu` (or `Namba_Kuu`
  mixed with `Namba`), arithmetic stays in `BigInt` — preserving exact integer division/modulo,
  not silently promoting to decimal; a `Namba_Sahihi` on either side promotes both to
  `BigDecimal`. `/`/`%` by zero return `EvalError::DivByZero` (the same clean error path
  ordinary `Namba` division-by-zero doesn't even need anymore — this is a different call site).
  `**` uses `BigInt::pow(u32)` / `BigDecimal::powi(i64)` — integer exponents only, since neither
  crate defines a general fractional big-number power.
  **The semantic analyzer needed the identical extension separately** — its own
  `Expr::Binary`/`BinaryOp::Add` type-checking (`core/parser/src/semantic/analyzer.rs`) is a
  wholly separate code path from the evaluator's runtime dispatch and required its own
  `Namba_Kuu`/`Namba_Sahihi`-aware arm before the plain-`Namba` checks, or `weka a: Namba_Kuu =
  ...; a + a` would be rejected at compile time (`SEM033`) despite the evaluator supporting it
  perfectly well at runtime.
- **Casting**: `Namba -> Namba_Kuu`/`Namba_Sahihi` is infallible (widening) — `Namba_Kuu`
  truncates toward zero (an integer type can't represent a fraction), `Namba_Sahihi` uses
  `BigDecimal::from_f64` so the exact IEEE-754 value round-trips rather than a lossy
  decimal-string reformat. `Namba_Kuu`/`Namba_Sahihi -> Namba` is fallible (`Chaguo<Namba>`),
  reusing the `Biti8`-style fallible-cast pattern exactly as anticipated — **this needed the
  analyzer's `Expr::Cast` type-inference extended too**: it previously hardcoded `fallible =
  s.starts_with("Biti") || s.starts_with("uBiti")` (target-type-name-only, no knowledge of the
  *source* expression's type), so a `Namba_Kuu -> Namba` cast's static type came out as `Namba`
  even though the runtime value was `Chaguo(Namba)` — the analyzer now also checks the source
  expression's inferred type when the target is `Namba`.
- **Literal syntax**: no lexer/grammar changes, as predicted — construction is exclusively via
  `namba_kuu_kutoka(neno) -> Tokeo<Namba_Kuu, Neno>` / `namba_sahihi_kutoka(neno) ->
  Tokeo<Namba_Sahihi, Neno>` (`core/evaluator/src/builtins/hisabati.rs`, gated behind `leta
  hisabati` like the rest of that module), parsing a decimal-digit string via each crate's own
  `FromStr`.

See `core/evaluator/tests/namba_kuu_sahihi.rs` for test coverage — notably
`namba_sahihi_addition`, which asserts `0.1 + 0.2 == 0.3` exactly (no f64 rounding artifact),
the actual point of the type.

## `Mfululizo<T>` — still not started

Genuinely blocked on the borrow-checker lifetime-strategy decision in
[phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md),
unless the `Rc`-based "fake it" stopgap is chosen deliberately.

## Cross-references

- [implementation-status.md](implementation-status.md#phase-i--catalyst-core-interpreter) —
  phase checklist entry this doc expands.
- [phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md) —
  `Mfululizo<T>`'s real blocker (lifetime-inference strategy).
- [docs/spec/04-type-system.md](../spec/04-type-system.md#data-shapes-additional) — normative
  type table this doc's four types are drawn from.
