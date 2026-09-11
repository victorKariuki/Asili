# Concurrency and async design: `tenda`/`njia`/`fungo`, `sawia`/`subiri`

**Status: not started (concurrency), blocked on concurrency (async).** Phase IV, per
[docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md)'s feature–phase
map, which also states the real ordering constraint directly: *"no async without runtime"* —
`sawia`/`subiri` cannot be implemented before `tenda`/`njia`/`fungo` establish a working
executor. This doc is forward-looking; there is no working concurrency/async code to describe,
only an important naming/model discrepancy to flag and the open design questions ahead.

## An existing discrepancy that needs resolving first

`core/evaluator/src/builtins/sambamba.rs` already exists, registers two functions
(`anza_mwendo`/`subiri_mwendo`), and is in the module whitelist (`core/parser/src/builtins.rs`'s
`BUILTIN_MODULE_NAMES`) — but **neither the module name `sambamba` nor either function name
appears anywhere in `docs/spec/*.md`**, confirmed by grep across the whole spec directory. The
spec instead describes `tenda`/`njia`/`fungo` (green threads / channels / mutex,
`docs/spec/05-standard-library.md`), a different naming scheme and — per `sambamba.rs`'s own
comments (`std::thread`/`tokio::task`) — a plausibly different execution model (OS threads or an
async-runtime task, not the green-thread ("tenda") model the spec names).

Both are currently **stubs that don't work** (`sambamba.rs`'s `anza_mwendo` always returns a
dummy handle `0` without spawning anything; `subiri_mwendo` always returns `Ok(Tupu)` without
joining anything — confirmed by reading the file, both paths are marked `FIXME(Phase IV)`), so
nothing is broken by either name today. But **this needs an explicit decision before real
concurrency work starts**, not silent resolution by whichever gets implemented first:

- **Is `sambamba` the spec's `tenda` under a different, earlier-chosen name?** If so, the spec
  should be updated to match the implementation's naming (`sambamba`/`anza_mwendo`/
  `subiri_mwendo`) rather than the reverse — or the implementation should be renamed to match
  the spec (`tenda`, and whatever `njia`/`fungo`-equivalent function names are chosen) before
  real logic is added to the stub, since renaming a working implementation is more disruptive
  than renaming a non-functional stub.
- **Or are they two distinct, intentionally separate surfaces** — `sambamba` as a lower-level
  OS-thread-based primitive, `tenda`/`njia`/`fungo` as a higher-level green-thread model built on
  top of (or instead of) it? Nothing in the spec or the stub comments suggests this was the
  intent, but it can't be ruled out without asking whoever added `sambamba`.

Given `sambamba.rs`'s comments explicitly reference `std::thread`/`tokio::task` — real OS
threads or an async-runtime task, not a green-thread scheduler Asili would need to implement
itself — the more likely reading is that `sambamba` was an earlier, simpler placeholder for what
the spec now specifies more precisely as `tenda`. This should be confirmed (or the spec updated)
before writing real scheduling logic into either name.

## `tenda`/`njia`/`fungo` (concurrency)

Per `05-standard-library.md`: `tenda` (green threads) as the execution primitive, `njia`
(channels — `njia.unda()`, `njia.tuma()`, `njia.pokea()`) for message passing, `fungo`
(mutex/lock — `fungo.funga()`, `fungo.fungua()`) for shared-state protection. The spec's own
hedge: *"Exact API is defined at implementation... Full concurrency model (M:N vs 1:1,
happens-before) is fixed in [Resolved Decisions] when the feature is implemented"* — meaning
even the function signatures above are illustrative, not committed.

Open design questions, none resolved anywhere in the spec or code today:

- **Scheduling model: M:N (green threads multiplexed onto OS threads) or 1:1 (green thread ==
  OS thread)?** M:N needs a real scheduler/runtime (the "executor" the async dependency note
  refers to) — a substantial undertaking with its own design surface (work-stealing vs.
  cooperative, stack size/growth for each green thread — this interpreter already uses `stacker`
  for its own recursion depth handling, which is relevant prior art for stack-growth strategy).
  1:1 is far simpler to implement (`std::thread` underneath, roughly what `sambamba.rs`'s
  comments already sketch) but doesn't scale to large numbers of concurrent `tenda` the way a
  green-thread runtime would, and arguably isn't what "green threads" in the spec's own
  vocabulary implies.
- **`Value` thread-safety.** Every `Value` variant today assumes single-threaded, `Rc`-based
  sharing where sharing exists at all (`Kasha_GC<T>`'s `Rc<RefCell<Value>>`, see
  [kasha-gc-design.md](kasha-gc-design.md)) — `Rc`/`RefCell` are explicitly not `Send`/`Sync`.
  Any concurrency model needs either: values crossing `tenda` boundaries to be restricted to a
  `Send`-safe subset (excluding `Kasha_GC<T>` unless it grows an `Arc`/`Mutex`-backed variant),
  or a copy-on-send semantics that sidesteps the sharing question by never actually sharing
  `Value` data across threads. `sambamba.rs`'s own TODO comment already names this concern
  ("Send-safe Values").
- **`happens-before`/memory-consistency model** — the spec defers this explicitly ("fixed... when
  the feature is implemented"). This needs to be a real decision (sequential consistency for
  `njia`/`fungo` operations is the simplest, safest default) before any implementation, not an
  emergent property of whatever gets built first.
- **Handle representation.** `sambamba.rs`'s stub already sketches a `Mutex<HashMap<u64,
  JoinHandle<Value>>>` global registry with `Namba`-typed handle IDs (the same pattern
  `kiungo.rs`'s FFI stub sketches for library handles as `Anuani`) — a reasonable, already-
  precedented approach if `tenda` ends up 1:1-OS-thread-based; needs rethinking if M:N is
  chosen (a green-thread ID isn't an OS `JoinHandle`).

## `sawia`/`subiri` (async)

Explicitly gated on the above: `05-standard-library.md` states async's "full semantics are
deferred... they require an executor/runtime," and the roadmap's dependency note is direct: *"Do
not add async syntax before the VM or runtime can schedule tasks."* Nothing here can be
meaningfully designed independently of the `tenda` scheduling-model decision — an async
executor built on an M:N green-thread runtime looks very different from one built on 1:1 OS
threads plus a task queue (the latter is closer to what Rust's own `tokio`, already named in
`sambamba.rs`'s comments as a candidate dependency, provides out of the box).

One question that *can* be settled independently of the scheduler choice: **does `sawia`/
`subiri` desugar to `Tenda`/`njia` primitives, or is it a genuinely separate primitive requiring
its own `Value` variant (a `Future`-equivalent) and its own evaluator support (a poll-based or
callback-based execution model)?** The spec's phrasing ("Future surface for non-blocking
execution") suggests a `Future`-shaped abstraction distinct from `tenda`'s thread-shaped one, but
this isn't stated as a resolved decision anywhere in `08-resolved-decisions.md`.

## Order of work, if this phase starts

1. **Resolve the `sambamba`/`tenda` naming discrepancy** — this is a five-minute decision
   (confirm intent, update spec or rename the stub) that should happen before anything else here,
   since it costs nothing to resolve now and costs real rework later if left ambiguous.
2. Decide the scheduling model (M:N vs. 1:1) — the actual hard design decision, gates everything
   else in this doc.
3. Decide `Value` thread-safety strategy (Send-safe subset vs. copy-on-send vs. something else).
4. Implement `tenda`/`njia`/`fungo` for real (replacing the `sambamba.rs` stub or renaming it,
   per step 1's resolution).
5. Only then design `sawia`/`subiri` against the resulting executor.

## Cross-references

- [implementation-status.md](implementation-status.md) — Phase IV is listed as not started;
  this doc's `sambamba`/`tenda` discrepancy finding is new since that summary was last written
  and should be folded in.
- [kasha-gc-design.md](kasha-gc-design.md) — the existing `Rc<RefCell<Value>>` sharing pattern
  this doc's `Value` thread-safety question has to reconcile with (or explicitly exclude from
  cross-thread use).
- [docs/spec/05-standard-library.md](../spec/05-standard-library.md#concurrency-primitives-tenda-njia-fungo)
  / [docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md#dependency-notes)
  — normative concurrency/async surface and the explicit async-needs-runtime dependency note.
