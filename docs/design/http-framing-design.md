# HTTP/1.1 framing design

`mkondo_tumikia_http` + `.soma_bailisi` add real HTTP/1.1 request/response framing on top of the
`Mkondo` listener/worker-pool machinery. See `docs/design/http-server-design.md` for how this
fits into the larger stack, and `docs/design/faili-mkondo-design.md`/`docs/design/tls-design.md`
for the listener/pool and TLS layers this builds directly on.

## The supportive fix: `.soma_bailisi`, because `.soma()` can't do keep-alive

`.soma()` (`Value::Mkondo`'s existing read method) is `read_to_string` — it blocks until the
peer closes the connection (EOF). Phase 11's raw-bytes `mkondo_tumikia` never needed anything
else: `kazi_jina(mkondo: Mkondo) -> Tupu` handles exactly one request per accepted connection, so
"read until the peer is done" is the right contract there.

HTTP/1.1 keep-alive breaks that assumption structurally, not just as an optimization: a
compliant client may send a second request on the same connection without closing it, so the
server must be able to read *one* request's bytes (bounded by the header/body boundary and
`Content-Length`) and then read again for the next request — which is impossible if the read
primitive itself blocks until a close that keep-alive says shouldn't happen.

**`.soma_bailisi(kikomo: Namba) -> Tokeo<Neno, Neno>`** is the fix: a single, non-EOF-seeking
`Read::read()` call bounded to `kikomo` bytes, returning whatever was actually available
(possibly fewer bytes than requested, possibly an empty string on a zero-byte read that isn't
itself an error). This is additive — `.soma()` keeps its exact existing behavior and every
current caller (`mkondo_unganisha`'s tests, `examples/mkondo_server/`'s one-request-per-connection
contract) is unaffected. `.soma_bailisi` is exposed at the Asili level (for programs that want to
build their own bounded-read protocols), and the same underlying `MkondoStream::read` call is
used directly, Rust-side, by `http.rs`'s own request-parsing loop — not by calling the Asili
method through the interpreter, which would add unnecessary overhead for something internal to
the framing layer.

## Why a separate entry point, not a mode flag

`mkondo_tumikia_http` is a new, additive function alongside `mkondo_tumikia`, not a flag that
changes the existing function's behavior. The two have genuinely incompatible `kazi_jina`
contracts: raw-bytes `mkondo_tumikia` calls `kazi_jina(mkondo: Mkondo) -> Tupu` (the worker
function owns the whole connection and does its own I/O); framed `mkondo_tumikia_http` calls
`kazi_jina(ombi: OmbiHttp) -> JibuHttp` (the framing layer owns request parsing and response
writing; `kazi_jina` only computes the response). Making one function serve both contracts would
mean either a runtime-checked union of two incompatible calling conventions, or breaking the
already-shipped, tested `mkondo_tumikia(mkondo: Mkondo) -> Tupu` contract
(`examples/mkondo_server/`) for existing callers. A separate function has one clear contract each.

Both entry points share the same worker-pool/timeout/TLS-branch machinery (`http_worker_loop`
mirrors `worker_loop`'s structure directly) — the framing layer is a distinct per-connection loop
body, not a reimplementation of the pool itself.

## `OmbiHttp`/`JibuHttp`: plain `Value::Struct`, no new runtime type

Following the same reflection-friendly pattern the JSON codec already established (`Value::Struct`
is a name+field-list pair a generic walk can handle with no per-type code), the parsed request and
the expected response are plain structs:

- `OmbiHttp { njia: Neno, anwani: Neno, vichwa: Kamusi<Neno, Neno>, mwili: Neno }` — method, path,
  headers, body.
- `JibuHttp { hali: Namba, vichwa: Kamusi<Neno, Neno>, mwili: Neno }` — status code, headers, body.

`http.rs`'s `request_to_value`/`value_to_response` functions never check the struct's *name* —
only that the expected fields (`njia`/`anwani`/`vichwa`/`mwili` on the way in; `hali`/`vichwa`/
`mwili` on the way out) are present with the right shapes. This mirrors `kutoka_json`'s own
leniency (see `json-codec-design.md`) rather than inventing a stricter contract for this one
feature.

**A real consequence of this choice**: because `Value::Struct` carries no compile-time-checked
shape in this interpreter's `FnContract` model, an Asili program using `mkondo_tumikia_http` still
needs its own `umbo OmbiHttp { ... }` / `umbo JibuHttp { ... }` declaration — purely so the
semantic analyzer accepts `kazi mtumishi(ombi: OmbiHttp) -> JibuHttp` as a parameter/return type
and type-checks field access (`ombi.njia`) against it (`SEM098`/`SEM099` otherwise). The runtime
values `http.rs` constructs and consumes never actually go through `umbo`-based construction
machinery — the declaration exists only to satisfy the type-checker. Both
`core/evaluator/tests/mkondo_http.rs` and `examples/http_server/src/kuu.as` include the matching
`umbo` declarations for exactly this reason.

## Request parsing: `httparse` over a bounded-read accumulation loop

`read_request` (`http.rs`) accumulates bytes into a growing buffer via repeated bounded reads
(the same primitive `.soma_bailisi` exposes), re-parsing the accumulated buffer with
`httparse::Request::parse` after each read until it reports `Status::Complete` (returns the
header/body boundary offset) or a hard parse error. Once headers are complete, `Content-Length`
(if present) determines how many more body bytes to read past that boundary before the request
is considered fully received.

A hard cap (`MAX_REQUEST_BYTES`, 1 MiB) on total accumulated size fails the request with `400`
rather than growing the buffer without limit on an adversarial peer sending an endless stream of
incomplete headers — the same resource-exhaustion concern the original production-readiness
survey raised, applied here to the framing layer's own accumulation buffer.

## Scope boundaries

The original three scope cuts below (chunked `Transfer-Encoding`, pipelining, `Expect:
100-continue`) have since been implemented — see "What changed" further down. Only HTTP/2 remains
out of scope.

- **HTTP/1.0 and HTTP/1.1 request lines only.** No HTTP/2 — an entirely different, binary framing
  protocol (HPACK header compression, stream multiplexing) that isn't a "finish this later"
  extension of what's built here; it would be new, separate work on top of a different wire
  format entirely.

## What changed since the original pass (issues #20, #21, #22)

- **Chunked `Transfer-Encoding` is decoded, not rejected.** `decode_chunked_body`
  (`core/evaluator/src/builtins/http.rs`) parses the real `<hex-size>[;ext]\r\n<data>\r\n`
  chunk framing (RFC 7230 §4.1), terminated by a zero-size chunk, handling chunk-size-line
  extensions (ignored) and trailer headers (consumed off the wire so the connection stays in
  sync, but discarded — this codebase has no trailer-header concept to expose them through). The
  decoded body is spliced back into the same buffer the `Content-Length` path already used, so
  everything downstream (pipelining's leftover-byte carry-over, the `Neno` body conversion)
  treats it identically to a fixed-length body. As before, `kazi_jina`'s response body is always
  a `Neno` already fully in memory, so the server still never needs to *emit* chunked encoding —
  this only ever affected incoming request bodies.
- **Pipelining is supported.** `read_request` now takes a `carry: &mut Vec<u8>` buffer, owned per
  connection by `http_worker_loop` — any bytes read off the wire past the just-parsed request's
  boundary (because a peer sent a second request in the same TCP segment/`write` call) are
  stashed into `carry` instead of being silently dropped, and the next `read_request` call seeds
  its accumulation buffer from `carry` before touching the network again. A fully pipelined next
  request is answered with zero additional reads.
- **`Expect: 100-continue` gets a real intermediate response.** Once `read_request` knows headers
  are complete, it checks for `Expect: 100-continue` and, if present, writes
  `HTTP/1.1 100 Continue\r\n\r\n` directly to the stream (guarded to fire at most once per
  request) before falling through to read the body. Not fully RFC 7231 §5.1.1-faithful (a
  spec-perfect server would only continue if it already intends to accept the body, which would
  need consulting `kazi_jina`/routing before the body is read — this pipeline is strictly
  headers-then-body-then-dispatch) — matching how minimally the rest of this framing pass is
  already scoped, it always continues once the header is present, which is correct for the
  common case (an endpoint that will accept the upload).

`connection_keep_alive` implements the version-dependent default correctly: HTTP/1.1 defaults to
keep-alive unless `Connection: close` is present; HTTP/1.0 defaults to close unless
`Connection: keep-alive` is explicitly present (the inverse default, per the HTTP/1.0 spec, which
predates persistent connections as a default).

## Response writing

`write_response` builds a literal `HTTP/1.1 <status> <reason>\r\n<headers>\r\nContent-Length:
<n>\r\n\r\n<body>` byte sequence and writes it directly — no dependency on `httparse` or any other
crate for the write direction, since constructing a well-formed status line and header block is
straightforward string formatting, unlike parsing arbitrary client input. `Content-Length` is
computed automatically from the response body's length unless the `kazi_jina`-supplied headers
already include one (letting a caller override it deliberately, though the common case needs no
explicit header at all).

## Error responses

Every failure path in `read_request` returns a typed `ParseOutcome` the worker loop turns into a
real HTTP error response rather than closing the connection silently or hanging:

- `ParseOutcome::BadRequest` (a malformed request line, invalid headers, a request that exceeds
  `MAX_REQUEST_BYTES`, a malformed chunk-size line, or the peer closing the connection with an
  incomplete request/chunk already buffered) → `400 Bad Request`. `ParseOutcome::NotImplemented`
  (previously used for chunked `Transfer-Encoding` → `501`) no longer exists — chunked bodies are
  decoded now, not rejected.
- `ParseOutcome::ConnectionClosed` (the peer closed the connection cleanly with **no** partial
  request buffered — i.e., between requests on a keep-alive connection, or the very first read on
  a new connection) is not an error at all — the worker loop exits its keep-alive loop and moves
  on to accept the next connection, exactly the expected behavior at the end of a keep-alive
  session.
- A `kazi_jina` invocation that itself errors (an `EvalError` from `run_function`), or whose
  return value doesn't match the expected `JibuHttp` field shape, produces a `500 Internal Server
  Error` rather than propagating a Rust-level panic or leaving the connection hanging.

## Tests

`core/evaluator/tests/mkondo_http.rs` (same role-flip pattern as `mkondo_sikiliza.rs`/
`mkondo_tls.rs` — the Asili program is the server, driven by real raw HTTP bytes written directly
over a `TcpStream` from the test, rather than trusting an HTTP client crate's own framing):

- `get_request_response_round_trips` — a full request/response cycle, asserting the exact status
  and body.
- `request_headers_and_body_reach_kazi_jina` — a `POST` with a custom header and a body, proving
  both actually reach `kazi_jina` through `OmbiHttp`'s `vichwa`/`mwili` fields.
- `keep_alive_serves_two_requests_on_one_connection` — two requests sent over one connection with
  no reconnect between them, both answered correctly — proves the per-connection loop in
  `http_worker_loop` actually loops rather than closing after one request.
- `chunked_transfer_encoding_request_body_is_decoded` / `chunked_request_with_multiple_chunks_and_extension_is_decoded`
  — real chunk decoding (single chunk; multiple chunks with a chunk-size extension and a trailer
  header, proving those don't break framing).
- `pipelined_requests_are_both_answered_in_order` — two full requests written in a single
  `write_all` call (no read between them), both answered correctly and in order on the same
  connection with zero extra network reads for the second one.
- `expect_100_continue_gets_an_intermediate_response` — a real interim `HTTP/1.1 100 Continue`
  read directly off the wire before the client sends its body, followed by the real final
  response.
- `connection_closed_mid_body_is_rejected_with_400_not_a_hang` — a request declaring more body
  bytes than are actually sent, followed by the client closing its write half: resolves
  immediately with `400`, since there's nothing left to wait for once the peer has definitively
  finished sending (distinct from a merely slow/stalled peer, which is instead covered by the
  per-connection read timeout already proven directly in
  `mkondo::tests::connection_timeout_is_set_on_accept` — not re-proven here via a real 30-second
  wait, which would make the test suite itself slow for a property that test already covers).
- `malformed_request_line_is_rejected_with_400` — garbage input, not a panic.

Verified live, end-to-end, against the actual compiled `pata-cli` binary with real `curl`
requests (`GET /`, `POST /echo`, a 404 path, an explicit keep-alive check via `curl --http1.1`
requesting two URLs in one invocation, `curl -H "Expect: 100-continue"` showing a real
`100 Continue` before the `200 OK`, and a real `curl -H "Transfer-Encoding: chunked"` upload
correctly echoed back) — the same verification standard applied to every prior phase's example.
