# `Faili` / `Mkondo` / `Kumbukumbu<T>` design

`Faili` (file handle), `Mkondo` (TCP stream handle), and `Kumbukumbu<T>` (heap box) are the
"resource handles" concept from
[docs/spec/05-standard-library.md](../spec/05-standard-library.md#resource-handles-and-traits).
`Faili`/`Mkondo` own a real OS resource and close it deterministically on drop; `Kumbukumbu<T>`
owns no OS resource at all and is closer in spirit to a plain `Box`.

## Why a destructor mechanism was the first decision, not the types

The obvious design — mirror `Kasha_GC<T>`'s "no special drop glue needed" story — does not work
for `Faili`/`Mkondo`, and confirming why came before writing any type code.

`Env::drop` (`core/evaluator/src/env.rs`) is `scope.remove(name)`, called only from explicit
`tupa`. But `Env::pop_scope` — called at ordinary block/function exit — is a bare
`self.scopes.pop()`, which drops the whole `HashMap<String, Value>` directly and **never calls
`Env::drop`**. So a destructor hook placed on `Env::drop` would only fire for explicit `tupa`,
silently leaking any `Faili`/`Mkondo` that goes out of scope the normal way (the overwhelmingly
common case — most files aren't explicitly `tupa`'d, they're just used inside a block).

The fix: don't hook `Env`/`Stmt::Drop` at all. Give the OS-handle wrapper type its own real Rust
`Drop` impl. Then cleanup fires automatically whenever the wrapping `Value`'s Rust-level
ownership ends — via `pop_scope`'s bare `HashMap` teardown, via explicit `tupa`, or via
panic-unwind — because all three paths end in ordinary Rust value drop regardless of which one
removed the last reference. This is exactly how `Kasha_GC<T>`'s `Rc` refcount decrement already
works today: not a designed Asili-level hook, an *incidental* consequence of normal Rust drop
semantics. `Faili`/`Mkondo` make that mechanism load-bearing instead of incidental.

## The `Value` variants and their handle types

`core/evaluator/src/value/mod.rs`:

```rust
Faili(Rc<RefCell<FailiHandle>>),
Mkondo(Rc<RefCell<MkondoHandle>>),
Kumbukumbu(Box<Value>),

pub struct FailiHandle(pub Option<std::fs::File>);
impl Drop for FailiHandle {
    fn drop(&mut self) { let _ = self.0.take(); }
}

pub struct MkondoHandle(pub Option<std::net::TcpStream>);
impl Drop for MkondoHandle {
    fn drop(&mut self) { let _ = self.0.take(); }
}
```

- **`Rc<RefCell<_>>`, not `Arc<Mutex<_>>`**: this interpreter is single-threaded by construction
  today (see `docs/design/concurrency-async-design.md`'s findings on `Env`/`Runtime` being
  reference-based, not `Arc`-shared). No need to pay thread-safety cost for a value that can't
  cross a thread boundary yet. If real concurrency (`tenda`) needs to pass a `Faili`/`Mkondo`
  across threads later, that's the point to reconsider — not before.
- **`Option<File>`/`Option<TcpStream>`, not a bare handle**: lets `.funga()` (explicit close) and
  the `Drop` impl share one code path (`.take()`), and makes double-close — whether two explicit
  `.funga()` calls, or a `.funga()` followed by scope-exit — a safe no-op instead of a panic or
  double-free. `.soma()`/`.andika()` check `is_some()` and return `Tokeo(Kosa(...))` on an
  already-closed handle rather than panicking.
- **`Kumbukumbu(Box<Value>)` has no destructor, deliberately**: it owns no OS resource. Plain
  `Box` drop (recursively dropping its contents) and the existing move/clone semantics every
  other `Value` variant already has are sufficient — adding a `Drop` impl here would be pure
  ceremony with nothing to actually clean up.

**Equality is by identity for `Faili`/`Mkondo`** (same reasoning as `Kasha_GC`'s `Rc::ptr_eq`):
two handles are `==` iff they're the same underlying OS resource, not two separately-opened
handles to the same path/address. `Kumbukumbu` is structural (`Box<Value>`'s own `PartialEq`),
since it's just a value wrapper with no identity concept of its own.

## The types

`ValueType::Faili`, `ValueType::Mkondo` (both unit — non-generic, unlike `Kasha_GC<T>`), and
`ValueType::Kumbukumbu(Box<ValueType>)` (`core/parser/src/ast.rs`). Type-annotation parsing
(`core/parser/src/semantic/types.rs`) recognizes `"Faili"`/`"Mkondo"` as exact matches and
`"Kumbukumbu<...>"` as a generic prefix, mirroring `Kasha_GC<T>`'s pattern for the generic case.

## Not opt-in — a deliberate deviation from `Kasha_GC<T>`'s pattern

`Kasha_GC<T>` is gated behind explicit `leta kasha_gc` because it's presented in the spec as an
optional, separate managed-memory module layered on top of the default ownership model. Resource
handles are different: the spec places them directly under "Resource handles and traits," inside
the existing `Mfumo`/`Faili` sections, with no opt-in framing. So:

- `faili_fungua` is exported from the **existing** `"faili"` module (`core/parser/src/
  builtins.rs::faili_exports()`) — reachable via the same `leta faili` a program already needs
  for `soma_faili`/`andika_faili`, not a new module name.
- `mkondo_unganisha` is exported from the **existing** `"mfumo"` module
  (`mfumo_exports()`) — reachable via `leta mfumo`.
- `kumbukumbu_unda` is exported from **`msingi`** (`msingi_exports()`) — always in scope, no
  `leta` needed at all, the same as `orodha()`/`kamusi()`/`jozi()`.

No new entries were needed in `core/parser/src/builtins.rs`'s `BUILTIN_MODULE_NAMES` or
`core/parser/src/semantic/analyzer.rs`'s parallel allowed-`leta`-targets list — both already
contain `"faili"`/`"mfumo"`/(msingi is always-seeded, not gated by either list).

## Methods

Registered as method-dispatch arms on `(Value::Faili(cell), "<method>")` /
`(Value::Mkondo(cell), "<method>")` / `(Value::Kumbukumbu(v), "<method>")` in
`core/evaluator/src/eval/expr.rs`, with matching type contracts in
`core/parser/src/semantic/analyzer.rs` (`ValueType::Faili`/`Mkondo`/`Kumbukumbu(_)` added to the
`is_builtin` receiver-type set alongside `KashaGC`/`Neno`/`Orodha`/etc.):

| Type | Method | Evaluator behavior | Return type |
|---|---|---|---|
| `Faili`/`Mkondo` | `.soma()` | Reads the handle to a `String` via `Read::read_to_string`. `Tokeo(Kosa(...))` on I/O error or if already closed. | `Tokeo<Neno, Neno>` |
| `Faili`/`Mkondo` | `.andika(data)` | Writes `data` via `Write::write_all`. Same error handling as `.soma()`. | `Tokeo<Tupu, Neno>` |
| `Faili`/`Mkondo` | `.funga()` | `cell.borrow_mut().0.take()` — explicit close. Idempotent. | `Tupu` |
| `Kumbukumbu<T>` | `.pata()` | `(**v).clone()` — a clone of the boxed value. | `T` |

**`Kumbukumbu<T>` deliberately has no `.weka()`.** A method call's receiver (`recv` in
`eval/expr.rs`'s `Expr::MethodCall` handling) is already an *evaluated, owned clone* of whatever
`Env::get` returned for the binding — true for every builtin method call in this interpreter, not
specific to `Kumbukumbu`. For `Kasha_GC<T>`, mutating through that clone still works because the
clone is an `Rc` pointing at the same shared `RefCell` — mutation is visible through every other
handle. `Kumbukumbu<T>`'s `Box<Value>` has no such sharing: mutating the method call's local
clone's box would be invisible to the original `weka`-bound value, silently doing nothing useful.
Rather than ship a method that looks like it mutates but doesn't, `Kumbukumbu<T>` only supports
`.pata()` (read) and whole-value reassignment (`weka k = kumbukumbu_unda(newval)`). If in-place
mutation through a `Kumbukumbu<T>` is ever needed, that is what `Kasha_GC<T>` is for — the two
types are not meant to converge.

## Constructors

Free functions, not method calls, since they don't yet have a receiver:

- `faili_fungua(njia: Neno, hali: Neno) -> Tokeo<Faili, Neno>` — `hali` is `"soma"` (read-only,
  errors if the file doesn't exist), `"andika"` (write, truncating/creating), or `"ongeza"`
  (append, creating if absent). An unrecognized `hali` string returns `Tokeo(Kosa(...))` rather
  than panicking or silently defaulting.
- `mkondo_unganisha(anwani: Neno) -> Tokeo<Mkondo, Neno>` — connects a TCP client stream to
  `anwani` (e.g. `"127.0.0.1:8080"`). No listening-socket/server variant exists yet — out of
  scope for this pass; the spec text ("network stream or socket") doesn't commit either way, and
  nothing currently in the spec or examples calls for a server socket.
- `kumbukumbu_unda(thamani) -> Kumbukumbu<T>` — wraps `thamani` in a new box.

## `wasm32` gating

`Faili`'s and `Mkondo`'s constructors follow the exact `#[cfg(any(not(target_arch = "wasm32"),
feature = "wasm-wasi"))]` / `#[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]`
split every other filesystem/network builtin in this codebase already uses (see
`core/evaluator/src/builtins/faili.rs`'s existing `soma_faili`/`andika_faili` for the pattern) —
a browser build with neither `wasm-wasi` nor a hypothetical future socket-polyfill feature
returns `Tokeo(Kosa("... haipatikani kwenye kivinjari"))` instead of attempting a real filesystem
or network call.

## Tests

`core/evaluator/tests/faili.rs`:

- `write_then_read_round_trips` — open for write, write, close, reopen for read, read back —
  proves the basic constructor/method contract end-to-end through the real compile pipeline.
- `explicit_funga_closes_the_handle` — writing after `.funga()` returns `Tokeo(Kosa(...))`, not
  a panic or silent no-op.
- **`scope_exit_without_explicit_tupa_still_closes_the_handle`** — the test that actually proves
  the design decision above: opens a file inside an inner block with no `tupa`/`.funga()` at
  all, lets it fall out of scope normally, then opens the same path again for write and confirms
  the second open/write/read round-trips cleanly — which would not be reliable if the first
  handle's `Drop` hadn't already released the OS resource by the time the second open runs.
- `faili_requires_leta_faili` — `faili_fungua` is unknown without `leta faili` (confirms it's
  gated the same way `soma_faili`/`andika_faili` already are, not left ungated by accident).
- `unknown_mode_returns_kosa`, `opening_missing_file_for_read_returns_kosa` — error paths return
  `Tokeo(Kosa(...))`, not panics.
- `double_funga_is_a_safe_no_op` — two explicit `.funga()` calls in a row don't panic.

`core/evaluator/tests/mkondo.rs` mirrors the same shape against a real local `TcpListener`:
`connect_write_and_server_receives_it` (a background thread accepts and reads what the Asili
program wrote), `connecting_to_a_closed_port_returns_kosa`, `double_funga_is_a_safe_no_op`,
`mkondo_requires_leta_mfumo_or_resolved_module`.

`core/evaluator/tests/kumbukumbu.rs`: `pata_returns_the_boxed_value`,
`no_leta_needed_it_is_always_in_scope` (confirms the msingi-default decision above),
`reassigning_the_binding_does_not_affect_a_prior_pata_snapshot`.

Also exercised end-to-end at `examples/faili/src/kuu.as`.

## Cross-references

- [docs/spec/05-standard-library.md](../spec/05-standard-library.md#resource-handles-and-traits)
  — normative stdlib entry.
- [kasha-gc-design.md](kasha-gc-design.md) — the sibling design doc this one's structure
  follows, and the type `Kumbukumbu<T>` is explicitly *not* trying to converge with (see
  "Methods" above).
- [implementation-status.md](implementation-status.md) — phase checklist entry this doc
  supersedes with detail.
- `Inasomeka`/`Inandikika` (traits `Faili`/`Mkondo` are meant to eventually implement) are not
  yet real — see the Sifa/traits work tracked separately; today's method dispatch is direct
  builtin dispatch, not a trait `impl`.
