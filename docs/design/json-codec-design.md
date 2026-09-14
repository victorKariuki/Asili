# `Value` ↔ JSON codec design

`kwa_json`/`kutoka_json` (`core/evaluator/src/value/json.rs`, exposed from the existing `mfumo`
module) convert between `Value` and a JSON string. This is the foundation piece for any future
HTTP-server-shaped stack (see `docs/design/http-server-design.md`) — a request/response body
needs *some* wire format, and JSON is the obvious default.

## The exclusion set is stricter than, and independent from, `SendValue`'s

This is the one place two "which variants are safe to move" lists in the codebase disagree on
purpose, worth recording so a future reader doesn't try to unify them.

`SendValue` (`docs/design/concurrency-design.md`) excludes `KashaGC`/`Faili`/`Mkondo`/
`Kumbukumbu` — variants that hold `Rc`, not `Send`. The JSON codec excludes those same four
variants **and also** `NjiaTx`/`NjiaRx`/`Fungo` — variants that **are** `Send`-safe (they pass
`Value::try_into_send` cleanly; that's their entire purpose) but have no JSON representation at
all. A channel sender or a mutex handle isn't data — it's a live, stateful OS/runtime resource,
the same category `Faili`/`Mkondo` are excluded for, just via a different underlying mechanism
(`Arc<Mutex<_>>` instead of `Rc<RefCell<_>>`).

Concretely, `to_json`/`from_json` do **not** reuse `SendValue`/`try_into_send` — a fresh,
JSON-specific recursive walk was written instead, precisely to avoid silently inheriting
`SendValue`'s narrower exclusion set and letting a channel handle attempt serialization. A test
(`njia_and_fungo_rejected_even_though_send_safe`) makes this divergence explicit: it constructs
an `NjiaTx`, confirms `try_into_send()` succeeds on it (sanity-checking it really is `Send`-safe),
then confirms `to_json()` still rejects it.

## Cycle safety: depth cap, not a cycle detector

`KashaGC(Rc<RefCell<Value>>)` can form reference cycles at the Asili level
(`weka a = kasha_gc_unda(hamna); a.weka(a)`), but `KashaGC` is already excluded from JSON per the
rule above — so a cycle *through* `KashaGC` is moot for this codec; it can never be reached.

What the codec still needs a guard against: plain nesting depth, via `Struct`/`Orodha`/`Kamusi`
recursing arbitrarily deep. A hard cap (`MAX_DEPTH = 64`) makes both `to_json` and `from_json`
fail cleanly (`EvalError::Coded { kind: ErrorKind::BadInput, .. }` for the encode direction;
`from_json` degrades gracefully to `Hamna` past the cap rather than failing, since decoding
untrusted JSON should never panic) instead of stack-overflowing. This matters once a future HTTP
listener (`docs/design/http-server-design.md`) accepts JSON request bodies from an untrusted
network peer — the codec was written for that eventual caller, not only trusted in-program use.

## Per-variant encoding choices

- **`NambaKuu`/`NambaSahihi` → JSON string, not number.** Arbitrary precision doesn't round-trip
  through `f64`/JSON's number type. Matches the existing decimal-string convention their
  `kama Neno` casts already use (`eval/expr.rs`).
- **`Anuani` (raw memory address) → JSON string, not number.** Deliberate: a JSON number a client
  could do arithmetic on has no safe use for a raw address.
- **`Herufi` → single-character JSON string.** JSON has no char type.
- **`Chaguo(Some(v))` → the inner value directly; `Chaguo(None)` → `null`.** Not wrapped in a
  tag — `Kuna`/`Hamna` collapse to "present or `null`," matching how most JSON APIs represent an
  optional field.
- **`Tokeo(Ok(v))`/`Tokeo(Err(v))` → `{"Sawa": v}` / `{"Kosa": v}`.** Tagged, since unlike
  `Chaguo` there are two distinct non-null cases to distinguish.
- **`Struct(name, fields)` → a flat JSON object of its fields, name discarded.** This is the
  reflection-friendly path that needed no new type-system feature: `Struct` is already a
  name+field-list pair, so a generic recursive walk handles any struct without per-type code.
- **`Enum(enum_name, variant, data)` → `{"aina": variant, "data": data}`** (data omitted for a
  unit variant). `enum_name` itself is discarded, same reasoning as `Struct`.
- **`Kamusi` keys → stringified.** A JSON object's keys are always strings; `MapKey::Namba`/
  `Ukweli`/`Herufi` keys are stringified rather than rejected, since a `Kamusi<Namba, _>` is
  common enough to be worth supporting on the wire (as `{"1": ...}`-shaped output) — matching how
  `JSON.stringify(Map)` and most JSON-object-from-map codecs behave elsewhere.
- **`from_json` on a JSON object → always `Kamusi<Neno, Value>`.** A JSON object has no
  distinction between "this was an Asili `Struct`/`Tokeo`/`Enum`" and "this was a `Kamusi`" — a
  caller that needs a specific `Struct` back constructs one from the decoded `Kamusi`'s fields.
  This is also why `kutoka_json`'s declared static return type
  (`core/parser/src/builtins.rs`) is `Tokeo<Kamusi<Neno, Unknown>, Neno>`, not bare `Unknown` —
  the analyzer rejects method calls on a statically-`Unknown`-typed receiver (`SEM039`), so a
  concrete `Kamusi` return type is what makes `.pata(...)` on a decoded object actually
  type-check for the common "parse a JSON object" case. Other decoded shapes (an array decodes
  to `Orodha`, a scalar to `Namba`/`Neno`/etc.) still work correctly at runtime; only their
  static method/index calls would need an explicit cast, since the declared type can't vary
  per-call-site.

## Scoped out of v1

**Field rename/omit.** The wire format uses internal field names verbatim — no
`#[jina("...")]`-style rename attribute, no per-field skip. This is a real, intentionally-scoped
gap: a future React frontend expecting idiomatic camelCase JSON would see Asili's own field names
as-is. The extension point, if this is ever needed, is a struct-field attribute the codec's
`Struct` arm would consult — not a new subsystem.

## Builtin surface

`kwa_json(thamani) -> Tokeo<Neno, Neno>` and `kutoka_json(neno) -> Tokeo<Kamusi<Neno, Unknown>, Neno>`,
registered in `core/evaluator/src/builtins/json.rs`, exported from the existing `mfumo` module
(`core/parser/src/builtins.rs`'s `mfumo_exports()`) — no new `leta` target, matching this
codebase's established convention for core capabilities that aren't opt-in the way `Kasha_GC<T>`
deliberately is.

## Tests

`core/evaluator/src/value/json.rs`'s own `#[cfg(test)]` module: round-trip tests for every
JSON-safe variant, explicit rejection tests for each excluded variant (including the
`SendValue`-divergence test above), a depth-limit test, and `NambaKuu`/`Anuani` string-encoding
tests. `core/evaluator/tests/json.rs`: integration tests through the real
tokenize→parse→semantic_check→run_function pipeline — encoding a `Namba`, encoding a `Struct`
reflectively, decoding a JSON object into a `Kamusi` and reading a field back, rejecting invalid
JSON, a full round trip, and rejecting a `Kasha_GC` handle.
