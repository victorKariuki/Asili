# Networking design

Status: implemented (sockets, UDP, HTTP/1.1 and HTTP/2 server and client, WebSocket, HTTP/3).

## Layers

- **`asili-mtandao`** (`core/mtandao`) is the one network layer. It owns connecting, listening,
  TLS configuration (one crypto provider, aws-lc-rs), DNS, UDP, WebSocket, HTTP on hyper
  (`http::seva` server, `http::mteja` client), and HTTP/3 on quinn and h3 (`http::h3`).
- **The evaluator** (`core/evaluator/src/builtins/mkondo.rs`, `http.rs`, `http_mteja.rs`) maps
  those onto Asili values and builtins. It does no protocol work of its own.

## Concurrency

A thread runs one tokio current-thread event loop (`kazi_sawia`). `sawia` tasks are coroutines on
that loop, so a wait on the network, a channel or `lala` lets the thread's other tasks run.
Servers spawn one task per connection (raw) or per request (HTTP); a worker thread therefore
serves many connections. Values cross threads only through `tenda`, `njia` and `fungo`; sockets,
listeners and WebSockets are thread-local, except that a listener is a shared handle (stopping it
from any thread stops every worker).

## Decisions

- **hyper, not a bespoke parser**, for HTTP/1.1 and HTTP/2: framing, keep-alive, pipelining,
  `Expect: 100-continue`, and the HTTP/2 flow control are hyper's.
- **quinn and h3 for HTTP/3**, on the same rustls provider as TLS, so the program holds one
  crypto library. HTTP/3 is served on the listener's address; because UDP and TCP ports are
  separate, the HTTP/3 port is the TCP port number on UDP.
- **Redirects, retries, cookies and proxies are client policy** and live in `http_mteja.rs`
  (the client's `ChaguoHttp` options), not in the transport. HTTP/1.1, HTTP/2 and HTTP/3 answers
  all pass through the same loop.
- **WebSocket upgrades** go through hyper's upgrade mechanism; the handler gets a `MkondoWs` over
  the upgraded connection. Over HTTP/2 the upgrade is refused.
- **Limits are explicit**: request and response body limits, header and connect timeouts, a
  30-second per-read and per-write limit on raw connections, and a grace period when a server
  stops.

## Known limits

- HTTP/3 connections are pooled per thread and per server; a request on a pooled connection that
  fails is retried once on a new connection.
- Server-side HTTP/3 compresses answers only when the request accepts it, as HTTP/1.1 does.
- The wasm (browser) build has no sockets; every network builtin reports that it is unavailable.
