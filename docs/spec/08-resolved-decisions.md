# 8. Resolved Decisions (Conflicts and Open Questions)

Previous: [Execution and Roadmap](07-execution-and-roadmap.md) | [Overview](../SPECIFICATION.md)

---

These are the definitive resolutions for the Asili v1.1 **Substrate**, following **Explicit Failure** and **Zero-Cost Abstractions**.

---

## 9.1 Types and semantics

| Topic | Decision |
|-------|----------|
| **Tokeo** | **Tokeo\<T, E\>**. User-defined error types are supported; `KOSA(maelezo)` is the common case for simple errors. |
| **Namba → Neno** | Via trait **Onyesheka** (Display). Standard types (e.g. `Namba`) implement it; `x kama Neno` uses this. |
| **String length** | **Neno.urefu()** returns **grapheme clusters** (Lugha-Mama). **Neno.biti_ngapi()** returns **bytes** (Nguvu). |
| **Math fallibility** | **Infallible:** `jumla`, `tofauti`, `zao`, `duara`, `absolute`. **Fallible:** `gawio`, `kipeo`, `mizizi` return **Tokeo\<Namba, Kosa\>**. |
| **Literal default** | Integer literal `2026` has type **Namba (f64)** unless context requires a different numeric type. |
| **Floating-point edges** | `Namba` follows IEEE 754; `Ukomo`, `-Ukomo`, and `Siyo_Namba` are first-class numeric states for overflow/undefined floating-point results. |
| **Friction vocabulary** | `Mfuriko` names overflow and `Ufinyu` names underflow in diagnostics/error payloads for numeric operations on constrained/fixed-width types. |
| **Fixed-width arithmetic semantics** | Arithmetic on `Biti*`/`uBiti*` uses deterministic two's-complement wrapping in core operators. APIs that opt into checked arithmetic must surface overflow/underflow explicitly via `Tokeo` (e.g. `KOSA(Mfuriko)` / `KOSA(Ufinyu)`). |
| **Recoverable errors** | Recoverable failures must be expressed as data using **Tokeo\<T, E\>** (and optionally **Chaguo\<T\>**/`T?`), not as exceptions. |
| **Panic (`paparika`)** | `paparika` is an unrecoverable panic (halt). It cannot be caught in Asili and is reserved for critical conditions only; regular error handling must use `Tokeo`. |

---

## 9.2 Syntax and keywords

| Topic | Decision |
|-------|----------|
| **Bitwise / shift** | **Shift:** `sogeza_kushoto`, `sogeza_kulia`. **Bitwise:** `na_biti`, `au_biti`, `xor_biti`. |
| **Operator families** | Arithmetic (`+`, `-`, `*`, `/`, `%`, `**`), comparison (`==`, `!=`, `>`, `<`, `>=`, `<=`), logical (`na`, `au`, `siyo`), assignment (`=`, `+=`, `-=`, `*=`, `/=`) are part of core Signal. |
| **Bitwise NOT** | `siyo_biti` is the unary bitwise inversion operator for fixed-width integer/bit types. |
| **kwa loop** | **Iterator:** `kwa x katika orodha`. **Range:** `kwa i kutoka 0 hadi n`. Both supported. |
| **Unbounded loop form** | `wakati milele { ... }` is the canonical syntax for intentional infinite loops. |
| **Entry point** | **kazi kuu(hoja: Orodha\<Neno\>) -> Tupu**. CLI args always passed; may be ignored. |
| **Import syntax** | **Selective:** `leta mfumo::{chapisha, toka}`. **Full module:** `leta mfumo`. |

---

## 9.3 Naming and structure

| Topic | Decision |
|-------|----------|
| **Dereva vs Mfumo** | **Split.** **mfumo** = high-level System API (StdLib). **dereva** = internal HAL/FFI layer used to implement mfumo. |
| **Akili location** | For v1.1, a **submodule of Msingi**, under **Takwimu/Safu**. **Conditionally compiled** (excluded for minimal embedded targets). |

---

## 9.4 Process and tooling

| Topic | Decision |
|-------|----------|
| **Doc rejection** | **CI check.** `pata thibitisha` fails if any public `kazi` or `umbo` lacks a documentation string; patch rejected. |
| **Mainline branch** | **asili-mainline**. |

---

## 9.5 Casting and safety

| Topic | Decision |
|-------|----------|
| **kama on failure** | Returns **T?**. On failure (e.g. `1000 kama Biti8`), result is **Hamna**. Caller must handle nullability. |
| **Precedence** | **kama** binds **tighter** than arithmetic. So `a + b kama Namba` means `a + (b kama Namba)`. |

---

## 9.6 Memory strategy (Orodha allocation)

| Topic | Decision |
|-------|----------|
| **Standard mode** | When the OS or allocator denies a request (e.g. `Orodha::ongeza` cannot allocate), the runtime **Aborts**. No `Tokeo` is returned; failure is fatal. |
| **Embedded / strict mode** | In constrained or explicitly configured environments, **Orodha::ongeza** (and equivalent growth operations) may return **Tokeo\<Tupu, Kosa\>** on allocation failure. Memory exhaustion is then an explicit, handleable **Signal**. The mode is determined by target (e.g. embedded) or runtime/allocator configuration. |

---

## 9.7 Chaguo vs T?

| Topic | Decision |
|-------|----------|
| **Chaguo\<T\>** | Formal type with variants **Kuna(T)** and **Hamna**. |
| **T?** | Syntactic sugar for **Chaguo\<T\>**. **Hamna** is the value for the empty variant. One type, two spellings. |

---

## 9.8 Error propagation (jaribu, ?)

| Topic | Decision |
|-------|----------|
| **jaribu** | Block or construct for error propagation. |
| **?** operator | Unwraps **SAWA(thamani)** or returns the **KOSA** variant to the caller. Used with **Tokeo\<T, E\>**. Exact syntax (e.g. `jaribu { ... }` or `?` suffix) is fixed when implementing. |

---

## 9.9 Attributes

| Topic | Decision |
|-------|----------|
| **#[jaribio]** | Discovered by **pata jaribu** and run as unit tests. |
| **#[sharti(...)]** | Conditional compilation; code included only when the condition holds. |
| **#[kiunganishi]** | Marks FFI declarations (link to external C/C++ libraries). |
| **#[ndani]** | Optimization hint (e.g. inline). |
| **Resolution order** | After AST, before codegen. |

---

## 9.10 Ownership and borrowing

| Topic | Decision |
|-------|----------|
| **Ownership** | Every value has a single owner. Assignment and argument passing move ownership by default unless the type implements `Nakala` (Copy). After a move, the previous binding may no longer be used. |
| **Borrowing syntax** | `azima` and `azima_tenda` are the canonical keywords for immutable and mutable borrows. Symbols `&` and `&mut` are allowed shorthand. |
| **Reference types** | `Rejeo<T>` and `Rejeo_Tenda<T>` are the underlying immutable and mutable reference types; `azima`/`azima_tenda` desugar to these. |
| **Aliasing rules** | Many immutable borrows or one mutable borrow at a time; not both simultaneously. |
| **Drop (`tupa`)** | Owned values are dropped deterministically at end of scope. `tupa x` explicitly drops `x` early; `x` cannot be used afterwards. Resources such as `Faili` and `Mkondo` must release underlying OS/hardware handles on drop. |
| **Copy/clone** | Types implementing `Nakala` may be copied implicitly in limited contexts; other types must be cloned explicitly via `.nakala()`. |

---

## 9.11 Concurrency

| Topic | Decision |
|-------|----------|
| **Scheduling model** | **1:1** — one `tenda` spawns one real OS thread (`std::thread`), not an M:N green-thread scheduler. Chosen for buildability: M:N needs a real runtime (stack growth per green thread, a scheduler, a task queue) that nothing in this codebase provides yet, and the spec's own "exact API is defined at implementation" hedge commits to neither model. Revisit only if 1:1's per-thread overhead is a demonstrated problem for a real workload. |
| **Cross-thread values** | `tenda` arguments and `fungo`-wrapped values must be `Send`-safe. `Kasha_GC<T>`, `Faili`, `Mkondo` (or anything transitively containing one) are rejected with `Tokeo(Kosa(...))`, not silently allowed. `Kasha_GC<T>` stays `Rc`-based and single-threaded rather than switched to `Arc` — see [concurrency-design.md](../design/concurrency-design.md). |
| **`tenda`'s return value** | A spawned function's own return value does not cross back through `subiri_tenda` (which reports only `Tokeo<Tupu, Neno>` — did the thread finish or panic). Results are communicated back via `njia`. |
| **`fungo`'s lock/unlock** | `.funga()`/`.fungua()` are explicit and independent calls (not a Rust-style scoped guard, since Asili has no closures to scope a critical section with). `.pata()`/`.weka()` lock-act-unlock atomically in one call and are the safer default; `.fungua()` without a matching prior `.funga()` is a programming error, reported as a panic. |

---

## 9.12 HTTP-server-shaped stack (`Mkondo` listener, structured errors)

| Topic | Decision |
|-------|----------|
| **Listener concurrency model** | **Bounded thread pool**, not an async runtime. `mkondo_tumikia` extends `sambamba`'s existing 1:1-OS-thread model — a fixed number of long-lived worker threads, each running its own `accept()` loop on a shared listener — rather than introducing `tokio` (or any async runtime) as a second concurrency substrate alongside `tenda`/`njia`/`fungo`. Rejected specifically to avoid taking an implicit position on how `sawia`/`subiri` (still fully deferred per 9.11's ordering, and [07-execution-and-roadmap.md](07-execution-and-roadmap.md)'s "no async without runtime" constraint) eventually gets its own runtime. Ceiling: hundreds of concurrent connections, not tens of thousands — an accepted, documented tradeoff for reusing existing, already-tested primitives over a rewrite; revisit only with real usage data showing the ceiling is actually hit. See [http-server-design.md](../design/http-server-design.md). |
| **`mkondo_tumikia` blocking model** | Blocks the calling thread forever (joins every worker thread it spawns) — no separate stop/handle mechanism in this pass. Matches the simplest "the last statement in `kazi kuu` starts the server" shape; stopping the server means stopping the process. |
| **Per-connection data model** | The accept loop runs *inside* each worker thread, not on a shared main thread — so the accepted `TcpStream` is constructed and consumed entirely within one thread and never needs to cross a `SendValue` boundary (unlike `tenda`'s spawned-function arguments). `kazi_jina(mkondo: Mkondo) -> Tupu` receives a real, live `Mkondo` handle and owns the whole connection lifecycle itself via `.soma()`/`.andika()`/`.funga()`. |
| **`EvalError::Coded` is additive, not a restructure** | A new `Coded { kind: ErrorKind, message }` variant was added alongside the six pre-existing `EvalError` variants, which stay untouched. `ErrorKind` (`BadInput`/`NotFound`/`Conflict`/`Internal`/`Unavailable`) is deliberately transport-agnostic — `EvalError` is used by the REPL/CLI too, not only a future HTTP layer — with a `From<&EvalError> for ErrorKind` fallback mapping every pre-existing variant to `Internal`, so nothing already in the codebase needs to migrate to get a (conservative) status code out of a future HTTP layer. Precedent: future error-model additions to `EvalError` should default to this additive pattern rather than a breaking migration, unless a specific reason demands otherwise. |
| **TLS — implemented via `rustls`** | `tls_sanidi` + `mkondo_tumikia`'s optional `tls: Chaguo<TlsUsanidi>` parameter. `rustls` chosen over `native-tls` (pure Rust, avoids a system OpenSSL dependency that would complicate the WASM/cross-compile story). `MkondoHandle` became `Option<MkondoStream>` (an enum over plain/TLS streams, not a second `Value` variant) so `.soma()`/`.andika()`/`.funga()` stayed unchanged — TLS-transparent to `kazi_jina`. A TLS handshake failure drops only that connection, not the worker thread. `rustls`/`rustls-pemfile` are scoped to non-`wasm32` targets only (no raw sockets in a browser; the crypto backend doesn't cross-compile cleanly there). See [tls-design.md](../design/tls-design.md). |
| **HTTP/1.1 framing — implemented via `httparse`** | `mkondo_tumikia_http` — a new, **separate entry point** alongside `mkondo_tumikia`, not a mode flag, since the two have genuinely incompatible `kazi_jina` contracts (`kazi_jina(mkondo: Mkondo) -> Tupu` for raw bytes vs. `kazi_jina(ombi: OmbiHttp) -> JibuHttp` for framed). `httparse` (already transitively present via `tower-lsp`, made a direct dependency of `core/evaluator`) parses request lines/headers; response writing is hand-formatted (no crate needed — constructing a well-formed status line/header block is straightforward, unlike parsing arbitrary client input). Required `.soma_bailisi` (a new, additive bounded-read `Mkondo` method) first — `.soma()`'s read-to-EOF semantics are structurally incompatible with HTTP/1.1 keep-alive, which must read one request and then read again on the same connection. `OmbiHttp`/`JibuHttp` are plain `Value::Struct`s (the JSON codec's reflection-friendly pattern), checked by field shape only, never by struct name. Explicitly not implemented: chunked `Transfer-Encoding` (rejected with `501`), pipelining, `Expect: 100-continue`, HTTP/2. See [http-framing-design.md](../design/http-framing-design.md). |

---

## Deferred (algorithm/runtime details)

The ownership and borrowing language surface is fixed in 9.10. Concurrency's scheduling model and cross-thread value rules are fixed in 9.11. Deferred items are implementation details: the internal borrow-checker algorithm, the M:N-vs-1:1 question for a future higher-throughput scheduler if 1:1 proves insufficient, and full async runtime semantics for **sawia**/**subiri**.

---

## Implications

- **Type of `expr kama T` when fallible:** The expression has type **T?**, not T. Callers must handle `Hamna`.
- **Tokeo vs T?:** Use **Tokeo\<T, E\>** for operations that can fail with a *reason* (I/O, parsing). Use **T?** for "value or absent" (e.g. failed cast, missing env var).
- **Syntactic spec:** The Signal table includes `sogeza_kushoto`, `sogeza_kulia`, and `na_biti`, `au_biti`, `xor_biti`, `siyo_biti` as specified above.

---

Previous: [Execution and Roadmap](07-execution-and-roadmap.md) | [Overview](../SPECIFICATION.md)
