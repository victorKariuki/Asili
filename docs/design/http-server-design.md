# Minimal HTTP-server-shaped stack: what it needed, and in what order it was built

This doc ties together three otherwise-separate pieces of work — the `Value`↔JSON codec, the
structured `EvalError` addition, and the `Mkondo` listener/worker pool — as a single narrative:
what a minimal Asili program that looks like an HTTP backend actually needs, and why they landed
in this specific order. Each piece has its own, more detailed design doc; this one is the map
between them.

## The origin: a production-readiness survey, not a spec commitment

Unlike `sawia`/`subiri` (async), which the spec explicitly names and defers, "Asili as an HTTP
server" has no textual anchor anywhere in `docs/spec/`. This work started from a live
architecture discussion (could Asili back an HTTP API, paired with a React+WASM frontend?) that
turned into a direct-source-verified survey of what was actually missing. The survey's findings,
in dependency order, are what this doc — and the phases it covers — implements:

1. **No `Value`↔JSON codec.** Nothing to serialize a request/response body with at all.
2. **`EvalError` was six flat variants, no status-code-mappable kind.** `Unknown(String)`/
   `Panic(String)` swallow everything as an untyped string — no way for an HTTP layer to
   deterministically map a fault to 400 vs. 404 vs. 500 without string-matching.
3. **`Mkondo` was a blocking TCP *client* only.** No `listen`/`accept` capability existed at
   all — a server literally could not be written, at any level, before this work.
4. **`njia` was always unbounded.** A fast producer against a slow consumer grows memory without
   limit — a real resource-exhaustion risk, not just a server-specific one (see
   `docs/design/concurrency-design.md`'s njia section — closed in the same work window as this
   listener, but tracked as its own independent gap, not part of the HTTP-specific stack).
5. **`Kasha_GC<T>` had no cycle-breaking escape hatch.** A leaked reference cycle in a
   long-running server process has nowhere to go (see `docs/design/kasha-gc-design.md`'s
   weak-reference section — same story as (4), a real but HTTP-orthogonal gap closed alongside
   this work).

Items 1–3 are this doc's direct subject, in the order they were built:

## 1. `Value` ↔ JSON codec — the foundation

See `docs/design/json-codec-design.md` for the full design. The one fact that matters for
everything downstream: `to_json`/`from_json` have an exclusion set (`KashaGC`/`Faili`/`Mkondo`/
`Kumbukumbu`/`NjiaTx`/`NjiaRx`/`Fungo`) that is **deliberately stricter than, and independent
from,** `SendValue`'s. Conflating the two would have silently let a channel handle attempt JSON
serialization, since `NjiaTx`/`NjiaRx`/`Fungo` genuinely pass `Value::try_into_send`'s check
(they're `Send`-safe) while having no JSON representation whatsoever.

Exposed as `kwa_json`/`kutoka_json` from the existing `mfumo` module — no new `leta` target,
matching this codebase's established non-opt-in-for-core-capabilities convention.

## 2. Structured `EvalError` — additive, not a restructure

`EvalError::Coded { kind: ErrorKind, message: String }` was added as a new variant alongside the
six pre-existing ones, which stay untouched. `ErrorKind` (`BadInput`/`NotFound`/`Conflict`/
`Internal`/`Unavailable`) is deliberately transport-agnostic naming — `EvalError` is a
core-evaluator type the REPL and CLI use too, not only a hypothetical future HTTP layer, so it
doesn't know about HTTP status codes at all. A `From<&EvalError> for ErrorKind` impl gives every
pre-existing variant a conservative `Internal` fallback, so nothing already in the codebase needs
to migrate to get *some* status code out of a future HTTP layer — that mapping (`BadInput` → 400,
`NotFound` → 404, `Conflict` → 409, `Unavailable` → 503, `Internal`/unmapped → 500) lives in
whatever HTTP-framing layer eventually consumes it (Phase 13, deferred — see below), not in
`core/evaluator`.

The JSON codec's own rejection paths (excluded-variant, depth-limit) were migrated to use
`Coded { kind: ErrorKind::BadInput, .. }` directly, as a concrete demonstration that a new
builtin can adopt the mechanism without ceremony.

## 3. `Mkondo` listener + bounded worker pool — the fork in the road, resolved

See `docs/design/faili-mkondo-design.md`'s "Listening sockets and the worker pool" section for
the full `Value`/ownership design. The one decision worth restating here, since it's the
highest-leverage call in this whole stack:

### Bounded thread pool, not an async runtime

**Locked decision:** `mkondo_tumikia` extends `sambamba`'s existing 1:1-OS-thread model — a fixed
number of long-lived worker threads, each running its own `accept()` loop directly on a shared
`Arc<TcpListener>` — rather than introducing `tokio` (or any async runtime) as a second
concurrency substrate alongside `tenda`/`njia`/`fungo`.

**Why not async:** `docs/design/concurrency-design.md` already explains at length why `tenda`'s
arguments and return values can't cross a thread boundary as a plain `Value` (the `Rc`-based
variants aren't `Send`), and why the `SendValue` structural-mirror solution was chosen over an
`unsafe impl Send` wrapper. Introducing `tokio` here would mean building a *second*,
differently-shaped answer to that exact same problem (`Future`-based tasks instead of OS
threads, a different set of Send/Sync constraints, likely a second value-mirror type) — two
incompatible concurrency substrates living in one evaluator, doubling the surface area for the
"which variants are safe to move where" question `SendValue` and the JSON codec's exclusion set
already have to answer separately. It also would have meant taking a position on how `sawia`/
`subiri` (still fully deferred, per the spec's own "no async without runtime" ordering) eventually
gets its runtime — a decision this pass had no mandate to make.

**Why the tradeoff is accepted:** a bounded pool's ceiling is hundreds of concurrent connections,
not tens of thousands — a real, known limitation of this design, not an oversight. For a v1 that
reuses existing, already-tested primitives (the exact handle-registry/thread-spawn pattern
`sambamba.rs`'s `tenda` already established) over a rewrite, this is the right trade. Revisiting
it is explicitly future work, gated on real usage data showing the ceiling is actually hit — not
a decision to pre-empt now.

**The `Value` shape this produced:** `MkondoSikilizaji(Arc<TcpListener>)` — `Arc`, not `Rc`, the
one exception in the Faili/Mkondo family, justified the same way `NjiaTx`/`NjiaRx`/`Fungo`
already are. `mkondo_tumikia` blocks forever (joins every worker thread), and each worker's
accept loop runs entirely inside its own thread — meaning the accepted `TcpStream` never needs
`SendValue` conversion, unlike `tenda`'s spawned-function arguments. `kazi_jina` receives a real,
live `Mkondo` handle and owns the connection with the same `.soma()`/`.andika()`/`.funga()`
methods the client-side handle already exposes.

## TLS — implemented

TLS is no longer deferred: `tls_sanidi` + `mkondo_tumikia`'s optional `tls` parameter add TLS
1.2/1.3 via `rustls`, exactly as recommended below when this was still an open item. See
`docs/design/tls-design.md` for the full design — the `MkondoStream` enum abstraction that made
`.soma()`/`.andika()`/`.funga()` TLS-transparent, the `Chaguo<T>` dual-representation bug this
surfaced, and the TLS `close_notify`-on-drop requirement. A plaintext-only deployment behind a
reverse proxy (nginx/Caddy terminating TLS) remains a legitimate option, but is no longer the
only one.

## HTTP/1.1 framing — implemented

Also no longer deferred: `mkondo_tumikia_http` (a new, additive entry point alongside the
raw-bytes `mkondo_tumikia`) adds real request-line/header/`Content-Length`-body parsing and
HTTP/1.1 keep-alive, via `httparse` exactly as recommended below when this was still an open
item. `kazi_jina`'s contract for this path is `kazi_jina(ombi: OmbiHttp) -> JibuHttp` — the
framing layer owns parsing and response writing; `kazi_jina` only computes the response. See
`docs/design/http-framing-design.md` for the full design — the `.soma_bailisi` bounded-read
primitive this needed (since `.soma()`'s read-to-EOF semantics are structurally incompatible
with keep-alive), the `OmbiHttp`/`JibuHttp` `Struct`-reuse decision, and the chunked/pipelining/
100-continue scope cuts (none of those three are implemented — see that doc for why each was cut
rather than "left for later").

## Cross-references

- `docs/design/json-codec-design.md` — full `Value`↔JSON codec design
- `docs/design/faili-mkondo-design.md` — full `Mkondo`/`Faili`/`Kumbukumbu` design, including the
  listener/worker-pool section this doc summarizes
- `docs/design/tls-design.md` — full TLS design (the `MkondoStream` enum, the `Chaguo<T>`
  dual-representation bug, the `close_notify`-on-drop requirement)
- `docs/design/http-framing-design.md` — full HTTP/1.1 framing design (`.soma_bailisi`,
  `OmbiHttp`/`JibuHttp`, the chunked/pipelining/100-continue scope cuts)
- `docs/design/concurrency-design.md` — the `SendValue`/1:1-thread rationale this doc's
  async-rejection reasoning cites directly
- `docs/spec/08-resolved-decisions.md` — records the bounded-thread-pool-over-async decision and
  the `EvalError::Coded`-is-additive precedent at the spec level
- `examples/mkondo_server/` — end-to-end example
