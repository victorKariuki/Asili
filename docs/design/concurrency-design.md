# Concurrency design: `tenda`, `njia`, `fungo`

**Status: implemented.** 1:1 OS-thread model (`std::thread`, not a real green-thread
scheduler), built on top of `core/evaluator/src/builtins/sambamba.rs` (kept that module name —
only the function names inside it were renamed to match spec vocabulary).

## The naming discrepancy, resolved

Before writing any real logic, `sambamba.rs`'s existing stub (`anza_mwendo`/`subiri_mwendo`)
matched no name anywhere in `docs/spec/*.md` — the spec instead names `tenda`/`njia`/`fungo`.
The stub's own comments already referenced `std::thread`/`tokio::task`, suggesting it was an
earlier placeholder for what the spec now calls `tenda`, not a deliberately separate primitive.
Resolved by renaming the *functions* to spec vocabulary — `anza_mwendo` → `tenda`,
`subiri_mwendo` → `subiri_tenda` — while keeping the module name `sambamba` (a reasonable
Swahili word for "parallel," and renaming a module is more disruptive than renaming two
never-actually-working function names). `subiri_tenda`, not the more obvious `subiri`, because
`subiri` is explicitly reserved by the spec for the future async `await` keyword
(`docs/spec/05-standard-library.md`'s "Async (sawia, subiri)" section) — reusing it here would
have been a real, spec-conflicting collision, not just an inconsistency.

## Scheduling model: 1:1, decided explicitly

M:N (green threads multiplexed onto OS threads) needs a real scheduler — stack growth per green
thread, a cooperative or preemptive yield point, a task queue — a multi-month project on its own
that this pass didn't attempt. 1:1 (one Asili `tenda` == one real OS thread via `std::thread`) is
what the existing stub already sketched, and the spec itself hedges ("exact API is defined at
implementation"), meaning nothing commits to M:N. Chosen as the pragmatic, buildable option;
revisit only if the 1:1 overhead genuinely becomes a problem for a real workload.

## Why `tenda`'s arguments and return value can't be a plain `Value`

This was the single hardest part of the implementation, worth recording precisely since it's not
obvious from the outside and cost real iteration to get right.

`Value` (`core/evaluator/src/value/mod.rs`) holds `Rc`-based interior-mutable variants —
`KashaGC`, `Faili`, `Mkondo` — deliberately not `Send` (that's the entire point of `Rc` over
`Arc`: cheaper, single-thread-only refcounting). `std::thread::spawn`'s signature is
`spawn<F, T>(f: F) -> JoinHandle<T> where F: Send + 'static, T: Send + 'static`. This bound is
checked on the **static type**, not a specific runtime value — so even though `tenda` already
validates its arguments don't *contain* a non-Send variant before ever spawning, that runtime
check does nothing to satisfy Rust's compile-time requirement: a closure that merely *captures*
a `Vec<Value>` fails to compile as a `thread::spawn` argument, full stop, regardless of what's
actually inside that `Vec` at runtime. The same problem hit the spawned function's *return*
value independently — `Result<Value, String>` isn't `Send` either, for the same reason.

**The `unsafe impl Send` route was considered and explicitly rejected.** A newtype wrapper
around `Value` with `unsafe impl Send`, justified by a runtime "does this actually contain a
non-Send variant" check, would compile and work — this is a real, sometimes-used Rust pattern.
But it was rejected here on safety, robustness, and scalability grounds: its soundness depends
on every future `Rc`-based `Value` variant remembering to be added to an exhaustive checker
function — and this codebase added three such variants (`KashaGC` pre-existing, `Faili`/`Mkondo`
new this session) in one work session alone. That's not a hypothetical risk; it's the observed
rate of change. A structural, compiler-checked solution scales with the codebase without relying
on anyone's memory.

**The chosen fix: `SendValue`** (`core/evaluator/src/value/mod.rs`), a mirror of `Value` that
structurally excludes the non-Send variants — it doesn't have a `KashaGC`/`Faili`/`Mkondo`/
`Kumbukumbu` case at all, so it's unconditionally `Send` by construction, checked by the
compiler, not by convention:

```rust
pub enum SendValue {
    Namba(f64), Neno(String), Ukweli(bool), Tupu, Hamna, Herufi(char),
    Wakati(f64), Anuani(u64), NambaKuu(BigInt), NambaSahihi(BigDecimal),
    Chaguo(Option<Box<SendValue>>), Tokeo(Result<Box<SendValue>, Box<SendValue>>),
    Orodha(Vec<SendValue>), Jozi(Box<SendValue>, Box<SendValue>),
    Kamusi(HashMap<MapKey, SendValue>), Seti(HashSet<MapKey>),
    Struct(String, Vec<(String, SendValue)>), Enum(String, String, Option<Box<SendValue>>),
    NjiaTx(Arc<Mutex<mpsc::Sender<SendValue>>>),
    NjiaRx(Arc<Mutex<mpsc::Receiver<SendValue>>>),
    Fungo(Arc<FungoCell>),
}
```

`Value::try_into_send(&self) -> Option<SendValue>` converts recursively, returning `None` the
first time it hits a non-Send variant anywhere in the structure (so a `Faili` buried inside an
`Orodha` inside a `Struct` field is caught, not just a bare top-level one).
`SendValue::into_value(self) -> Value` converts back, infallibly — every `SendValue` variant maps
to exactly one `Value` variant, so nothing can go wrong on the way back.

`tenda`'s arguments convert to `SendValue` before the closure captures them; the spawned
function's actual return `Value` is **discarded**, not converted back and returned through
`subiri_tenda` — see the next section for why.

## Why a spawned function's return value doesn't come back through `subiri_tenda`

Even with `SendValue` solving the argument-passing problem, the *return* value has one more
wrinkle: `crate::run_function` (the function `tenda` calls inside the spawned thread) returns a
real `Value`, not a `SendValue` — and converting it *after* the thread has already run doesn't
help, because the closure passed to `std::thread::spawn` still has to *type-check* as returning
something `Send` at the point it's defined, before any conversion could happen. The fix mirrors
the argument-side one: the spawned closure converts its own result to `Result<(), String>`
(discarding the `Value` entirely, keeping only success/failure) — `()` and `String` are
trivially, unconditionally `Send`, so this sidesteps the problem rather than working around it.

This means **`subiri_tenda(id)` reports `Tokeo<Tupu, Neno>` — did the thread finish cleanly or
panic — never the spawned function's actual return value.** A `tenda`-spawned `kazi` that wants
to communicate a result back must do so through `njia` (a channel), which is the idiomatic,
well-understood way real concurrent systems move data between threads anyway (the same shape Go,
Erlang, and Rust's own community conventions converge on) — not a narrower API than intended, a
better one. See `core/evaluator/tests/sambamba.rs`'s
`njia_send_and_receive_across_a_real_spawned_thread` for the full pattern.

## Why `njia`/`fungo` need to hold `SendValue`, not `Value` — twice

This tripped up the first implementation attempt and is worth flagging explicitly: it's not
enough for `NjiaTx`/`NjiaRx`/`Fungo` to be `Arc<Mutex<_>>`-wrapped instead of `Rc<RefCell<_>>`.
`Arc<Mutex<T>>`'s own `Send`/`Sync` impls require `T: Send` — wrapping a non-Send `Value` in
`Arc<Mutex<Value>>` does **not** make the whole thing `Send`; it just moves the same requirement
one level up, and the compiler correctly rejects it. So `Value::NjiaTx`/`NjiaRx` hold
`Arc<Mutex<mpsc::Sender<SendValue>>>`/`Arc<Mutex<mpsc::Receiver<SendValue>>>` — `SendValue`
payloads, not `Value` ones — and the mirror `SendValue::NjiaTx`/`NjiaRx` variants hold the
identical type (cloning the `Arc` directly when converting, no further transformation needed).

## `Fungo`: explicit `.funga()`/`.fungua()`, and why that needed `parking_lot`

The spec names `fungo.funga()`/`fungo.fungua()` (lock/unlock) as illustrative API, and this was
kept as the real one rather than substituting a `Kasha_GC`-style `.pata()`/`.weka()`-only
design — deliberately, per an explicit decision during implementation, accepting the tradeoff
below.

`std::sync::Mutex<T>`'s guard (`MutexGuard`) is scoped to a Rust lexical block by design — you
cannot acquire a lock in one function call and release it in a separate, later call, because the
guard's lifetime can't span that. Asili has no closures to scope a critical section with the way
Rust code normally would (`mutex.lock(); ...critical section...` — no `drop(guard)` equivalent
exists at the language level either), so a literal `.funga()`/`.fungua()` pair needs a lock
primitive that supports manual lock/unlock as two independent calls with no guard object at all.
`parking_lot`'s `RawMutex` (via the `lock_api` crate it's built on) is designed for exactly this.

`FungoCell` (`core/evaluator/src/value/mod.rs`) wraps a `parking_lot::RawMutex` plus an
`UnsafeCell<SendValue>` directly, with `lock()`/`unlock()`/`try_lock()`/`read()`/`write()`
primitives — `unlock()`/`read()`/`write()` are `unsafe fn` (matching `lock_api::RawMutex`'s own
contract: calling `unlock()` without holding the lock, or reading/writing without holding it, is
undefined behavior), with the safety obligation discharged by the method-dispatch call sites in
`core/evaluator/src/eval/expr.rs`, not by the caller of a safe public API. `FungoCell` gets its
own `unsafe impl Send + Sync` (justified: `raw` genuinely provides the mutual exclusion that
makes concurrent access to `data` sound, exactly the same safety argument `lock_api::Mutex`
itself relies on internally — this isn't a "trust me, it's fine" `unsafe impl Send`, it's the
standard, documented pattern for building a mutex-like type on `lock_api` primitives).

**Two access patterns exist, both real, for different purposes:**
- **`.pata()`/`.weka(v)` — self-contained, atomic, the usual way to use a `Fungo`.** Lock, act,
  unlock, all inside one call — no way to misuse this from Asili source, since there's no way to
  observe an intermediate locked-but-not-yet-acted-on state.
- **`.funga()`/`.fungua()` — explicit, for holding the lock across several operations.** This
  is the one place in the whole implementation with a real, acknowledged (not hidden) soundness
  gap: nothing tracks *which* Asili-level caller currently holds the lock, so a `.fungua()` call
  with no matching prior `.funga()` on the same handle can't be statically prevented — the
  dispatch code handles this by treating it as a real Asili-level programming error (`.fungua()`
  without holding first `try_lock()`s to check whether anything is actually held, and reports a
  panic rather than calling the truly-undefined-behavior `unlock()`-while-unheld), not silently
  corrupting state. This mirrors how a real, mismatched Rust `Mutex::unlock()` misuse would also
  be a logic bug the type system alone can't catch — the difference here is Asili has no compile-
  time enforcement at all for this pairing, only a runtime check. See
  `core/evaluator/tests/sambamba.rs`'s `fungo_fungua_without_funga_panics`.

## Cross-thread `Value` rejection

`Kasha_GC<T>`/`Faili`/`Mkondo` values (or anything transitively containing one — checked
recursively by `Value::try_into_send`) passed as `tenda` arguments, or wrapped in `fungo(...)`,
are rejected with a clear `Tokeo(Kosa(...))` error naming the problem, not a compile error and
not undefined behavior. `Kasha_GC<T>` stays deliberately single-threaded (`Rc`-based) rather than
switched to `Arc`-based sharing — that would be a real breaking change to its identity/perf
semantics (`Rc::ptr_eq` → `Arc::ptr_eq`, different atomics cost on every clone) taken on
speculatively, not because any concrete need demands it. If a program needs a `Kasha_GC<T>`'s
value shared across threads, `fungo` is the tool — wrap the value in a `Fungo` instead (after
converting through `.pata()`, since `Fungo` itself also only accepts `Send`-safe contents).

## Async (`sawia`/`subiri`) — still out of scope

The spec's own hard ordering constraint ("no async without runtime",
`docs/spec/07-execution-and-roadmap.md`) means `sawia`/`subiri` can't start until this
concurrency work is done and stable — which it now is, but building an async executor on top is
separate, larger, deliberately-not-attempted work. Track as a distinct future phase.

## Tests

`core/evaluator/tests/sambamba.rs`:

- `tenda_spawns_and_subiri_tenda_joins` — the basic spawn/join round trip.
- `tenda_unknown_function_returns_kosa` / `subiri_tenda_on_unknown_id_returns_kosa` — error paths
  for a bad function name / a bad or already-consumed handle id.
- `njia_send_and_receive_within_one_function` — channel round-trip with no real threading
  involved, isolating the channel mechanics from the threading mechanics.
- **`njia_send_and_receive_across_a_real_spawned_thread`** — the real end-to-end proof: a
  genuinely separate OS thread (via `tenda`) sends through a channel to the calling thread's
  `.pokea()`. Passes an `NjiaTx` *as an argument to `tenda`* — proving `SendValue` conversion
  handles the channel-handle-crossing-into-a-spawned-thread case, not just plain scalars.
- `njia_pokea_on_closed_sender_returns_kosa` — a real subtlety, worth flagging: the test needed
  an explicit `tupa p` on the `Jozi` holding the sender half, because Asili's evaluator
  deep-clones `Value`s freely and has no implicit end-of-scope drop before a function returns —
  simply not reading a binding again does **not** close the channel the way it would in Rust;
  verified live that omitting the `tupa` hangs `.pokea()` forever, exactly matching real `mpsc`
  semantics (`recv()` blocks until every `Sender` is dropped).
- `fungo_pata_weka_round_trip`, `fungo_funga_fungua_round_trip`,
  `fungo_fungua_without_funga_panics` — both `Fungo` access patterns, and the one documented
  misuse case.
- `kasha_gc_cannot_cross_tenda` — the cross-thread rejection check, exercised for real (not just
  unit-tested against `contains_non_send`/`try_into_send` directly).
- `sambamba_requires_leta` — `tenda` is unknown without `leta sambamba`.

Also exercised end-to-end at `examples/sambamba/src/kuu.as` and live-verified via a scratch
`pata-cli` project (spawn a worker thread that squares a number, channel the result back, plus a
`Fungo` read/write) before this design doc was written.

## Cross-references

- [docs/spec/05-standard-library.md](../spec/05-standard-library.md#concurrency-primitives-tenda-njia-fungo)
  — normative stdlib entry, now filled in with real signatures.
- [docs/spec/08-resolved-decisions.md](../spec/08-resolved-decisions.md) — the 1:1
  scheduling-model decision recorded there.
- [kasha-gc-design.md](kasha-gc-design.md) — why `Kasha_GC<T>` stays `Rc`-based and single-
  threaded rather than switched to `Arc` for this work.
- [faili-mkondo-design.md](faili-mkondo-design.md) — the other two `Rc`-based resource types
  `tenda`/`fungo` reject, for the same reason `Kasha_GC<T>` is rejected.
