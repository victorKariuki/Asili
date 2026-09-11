# `Kasha_GC<T>` design

`Kasha_GC<T>` is a minimal, opt-in reference-counted shared wrapper — the concrete realization
of the "managed/GC modules" concept in
[docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md). It is **not** a
tracing or cycle-collecting garbage collector; it's `Rc<RefCell<Value>>` with four methods, opt-in
via `leta kasha_gc`, layered on top of (not replacing) the language's default ownership/move
semantics.

## Why it exists

Every other `Value` in this interpreter has deep-copy clone semantics — `weka b = a` moves `a`
(per the ownership/borrowing rules in
[08-resolved-decisions.md](../spec/08-resolved-decisions.md#910-ownership-and-borrowing)), and
even where a value is shared it's shared by the language's own binding rules, not by aliasing a
mutable cell. `Kasha_GC<T>` is the escape hatch for the case ownership can't express cleanly:
multiple live bindings that need to observe each other's mutations to the same underlying value
(the classic "shared mutable state" pattern) without hand-rolling reference semantics.

## The `Value` variant

`core/evaluator/src/value/mod.rs`:

```rust
KashaGC(Rc<RefCell<Value>>),
```

No new plumbing was needed for sharing or refcounting beyond adding this one variant
(`core/evaluator/src/builtins/kasha_gc.rs`'s module doc comment spells this out explicitly):

- **Sharing:** `Value::clone()` on a `KashaGC` is `Rc::clone` — cheap, and it shares the
  allocation. Every read of a `weka`-bound variable already clones its `Value` when it's used, so
  `weka b = a.shirikisha()` (see below) naturally produces two `Rc` handles to the same
  `RefCell`.
- **Refcounting:** `tupa`/scope-exit already drops the `Value` normally, which drops the `Rc` and
  decrements its strong count via ordinary Rust ownership — no custom drop glue was written for
  `KashaGC` specifically.

**Equality is by identity, not contents** (`value/mod.rs`, `PartialEq` impl):

```rust
(Value::KashaGC(a), Value::KashaGC(b)) => Rc::ptr_eq(a, b),
```

Two handles are `==` iff they share the same underlying allocation. Comparing *contents* via
`RefCell`'s own equality would risk a panic if either handle is currently mutably borrowed
(`try_borrow_mut` in flight) — identity comparison sidesteps that entirely and matches the
intuitive meaning ("do these two handles point at the same shared cell") better than a structural
comparison would anyway.

## The type

`ValueType::KashaGC(Box<ValueType>)` (`core/parser/src/ast.rs`) — parsed from `Kasha_GC<T>` type
syntax (`core/parser/src/semantic/types.rs`, `parse_value_type`) and displayed the same way in
hover/diagnostics (`ast.rs`'s `Display` impl: `Kasha_GC<{T}>`).

## Opt-in wiring

`Kasha_GC<T>` is not in the default prelude. Three places gate it behind explicit `leta
kasha_gc`:

1. **Module whitelist** — `core/parser/src/builtins.rs`'s `BUILTIN_MODULE_NAMES` includes
   `"kasha_gc"` alongside the other builtin modules (`msingi`, `mfumo`, `hisabati`, etc.).
2. **Semantic analyzer's allowed-import list** — `core/parser/src/semantic/analyzer.rs` accepts
   `"kasha_gc"` as a valid `leta` target the same way it accepts the other builtin module names.
3. **Export table** — `core/parser/src/builtins.rs::kasha_gc_exports()` registers exactly one
   function, `kasha_gc_unda(v) -> Kasha_GC<T>` (`FnContract { params: [Unknown], ret:
   KashaGC(Unknown) }`) — this is the *only* export reachable via the module-import mechanism.
   Everything else (`.pata()`, `.weka()`, `.shirikisha()`, `.idadi()`) is an instance method,
   reached through the method-call path below, not through `leta kasha_gc`'s export table.

A program using `Kasha_GC<Namba>` (or calling `kasha_gc_unda`) without `leta kasha_gc` fails to
compile with the same "unknown module"-style diagnostic any other unimported builtin module
produces — verified by `kasha_gc_requires_explicit_import` (`core/evaluator/tests/kasha_gc.rs`).

## Methods

Registered as method-dispatch arms on `(Value::KashaGC(cell), "<method>")` in
`core/evaluator/src/eval/expr.rs` — not in the builtins export table, since they're called as
`handle.method()`, not as free functions. Each has a matching type contract in
`core/parser/src/semantic/analyzer.rs`'s method-call type-checking (`ValueType::KashaGC(_)` is
in the `is_builtin` set alongside `Neno`/`Orodha`/`Kamusi`/`Jozi`/`Chaguo`/`Tokeo`, so typo'd
method names on a `KashaGC` receiver get checked the same way as on any other builtin type):

| Method | Evaluator behavior | Return type |
|---|---|---|
| `.pata()` | `cell.try_borrow().map(\|v\| v.clone())` — a **snapshot clone** of the current contents, not a live view. Fails with a `Panic` if the cell is currently mutably borrowed (an in-flight `.weka()` call). | `T` |
| `.weka(v)` | `cell.try_borrow_mut()`, overwrites the slot with `v`. Fails with a `Panic` if the cell is already borrowed (another `.weka()`/`.pata()` in flight — this interpreter is single-threaded, so the only way to hit this is nested calls, e.g. inside a callback triggered by `.pata()`/`.weka()` itself). | `Tupu` |
| `.shirikisha()` | `Rc::clone(cell)` — the explicit, only sanctioned way to get a second handle to the same allocation. | `Kasha_GC<T>` |
| `.idadi()` | `Rc::strong_count(cell) - 1` — the `-1` corrects for the fact that `cell` itself, inside this method call, is a temporary clone of the receiver's `Rc` (evaluating the receiver expression clones it), which isn't a real independent handle a caller would count. Reports the number of live handles a caller would actually observe via other `weka` bindings. | `Namba` |

**Deliberately no `.tupa()`/explicit-drop method** — an existing `tupa <jina>` (drop) statement
already does the right thing: it removes the binding, which drops the `Value`, which drops the
`Rc`, decrementing the strong count through ordinary Rust ownership. No `KashaGC`-specific drop
logic exists or is needed.

## `weka b = a` vs. `.shirikisha()`

Plain `weka b = a` still **moves** `a` under this language's default ownership rules — wrapping
a value in `Kasha_GC<T>` does not silently turn `=` into a sharing operation. Sharing is only
achieved via the explicit `.shirikisha()` call, deliberately modeled on Rust's `Rc::clone()`
convention (an explicit method call, not an implicit `Copy`/`Clone`-on-assignment). This was a
considered design choice, not an oversight: making `Kasha_GC<T>` "just work" with plain `=` would
make its aliasing behavior invisible at the call site, and would special-case one type's move
semantics against the rest of the language's own rule that `weka b = a` always moves.

## Panics, not silent corruption, on `RefCell` conflicts

Because `.pata()`/`.weka()` use `try_borrow`/`try_borrow_mut` (not the panicking `borrow`/
`borrow_mut`), a conflict surfaces as this interpreter's normal `EvalError::Panic` with a Swahili
message (`"kasha_gc: pata: tayari inatumika..."` / `"kasha_gc: weka: tayari inatumika..."`) —
the same error channel `paparika` and other runtime panics use — rather than an unrecoverable
Rust-level `BorrowError`/process abort.

## Not a tracing/cycle-collecting GC

A self-referential `Kasha_GC<T>` (a `KashaGC` whose contents, transitively, hold a `KashaGC`
handle back to itself) leaks — the underlying `Rc` strong count never reaches zero, so the cell
is never freed. This is a known, accepted limitation of the reference-counting approach, not a
bug: implementing cycle detection/collection was explicitly out of scope for this minimal
version. If a program needs cyclic shared structures, `Kasha_GC<T>` as it exists today is the
wrong tool.

## Tests

`core/evaluator/tests/kasha_gc.rs`:

- `two_handles_share_a_mutation` — mutate through one handle, observe through the other (proves
  sharing, not copying).
- `refcount_increases_on_share_and_decreases_on_drop` / `refcount_reflects_two_live_shares` —
  `.idadi()` tracks live handle count correctly across `.shirikisha()` and `tupa`.
- `pata_returns_a_snapshot_not_a_live_view` — confirms `.pata()`'s clone semantics (mutating the
  cell after a `.pata()` call doesn't retroactively change the already-returned snapshot).
- `kasha_gc_requires_explicit_import` — using `Kasha_GC<T>`/`kasha_gc_unda` without `leta
  kasha_gc` fails to compile.
- `equality_is_by_shared_identity_not_contents` — two handles to the same cell are `==`; two
  handles to separately-constructed cells with identical contents are not.
- `weka_mutates_through_any_live_handle` — `.weka()` on any handle is visible through every other
  live handle to the same cell.

Also exercised end-to-end at `examples/kasha_gc/src/kuu.as` — construct, `.shirikisha()`, mutate
through one handle, read through the other, check `.idadi()` before/after a `tupa`.

## Cross-references

- [implementation-status.md](implementation-status.md#phase-ii--synthesis-lsp-wasm-tooling) —
  phase checklist entry this doc supersedes with detail.
- [docs/spec/05-standard-library.md](../spec/05-standard-library.md#kasha_gct-opt-in-managed-memory)
  — normative stdlib entry.
- [docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md) — the
  managed/GC-modules-layered-on-ownership memory strategy this implements one instance of.
- [docs/spec/08-resolved-decisions.md](../spec/08-resolved-decisions.md#910-ownership-and-borrowing)
  — the default ownership/move rules `Kasha_GC<T>` is explicitly layered on top of, not a
  replacement for.
- [mwalimu-design.md](mwalimu-design.md) / [package-manager-design.md](package-manager-design.md)
  / [wasm-driver-design.md](wasm-driver-design.md) / [sharti-design.md](sharti-design.md) —
  sibling design docs this one follows in structure/style.
