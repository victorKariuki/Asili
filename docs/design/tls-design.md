# TLS design

`tls_sanidi` + `mkondo_tumikia`'s optional 4th (`tls`) parameter add TLS 1.2/1.3 to the `Mkondo`
listener via `rustls` — the pure-Rust option, avoiding `native-tls`'s system OpenSSL dependency
(which would complicate this codebase's WASM/cross-compile story elsewhere). See
`docs/design/http-server-design.md` for how this fits into the larger HTTP-server-shaped stack,
and `docs/design/faili-mkondo-design.md`'s "Listening sockets and the worker pool" section for
the surrounding listener/pool design this extends.

## `MkondoStream`: an enum, not a second `Value` variant

The real prerequisite for TLS wasn't the handshake logic — it was making `MkondoHandle`
transparent to which kind of stream it wraps. `MkondoHandle` used to be a concrete
`Option<std::net::TcpStream>`; it's now `Option<MkondoStream>`:

```rust
pub enum MkondoStream {
    Wazi(std::net::TcpStream),
    Salama(Box<rustls::StreamOwned<rustls::ServerConnection, std::net::TcpStream>>),
}
```

`MkondoStream` implements `Read`/`Write` itself, delegating to whichever variant is active. This
means `.soma()`/`.andika()`/`.funga()` (`eval/expr.rs`'s `Value::Mkondo` method-dispatch arms)
needed **zero changes** — they already called through `Read`/`Write` on whatever `MkondoHandle`
held, so they became TLS-transparent for free. The alternative — a second `Value::MkondoSalama`
variant with its own parallel `.soma()`/`.andika()`/`.funga()` dispatch arms — was rejected
because it would double the method surface for something that should be invisible to `kazi_jina`:
a worker function reading a request shouldn't need to know or care whether the connection is
encrypted, any more than `mkondo_unganisha`'s client-side callers do today.

`Salama`'s payload is boxed — `rustls::StreamOwned` is large relative to a bare `TcpStream`, and
boxing keeps the common (plaintext) case of `Value::Mkondo`'s `Rc<RefCell<MkondoHandle>>` from
paying that size cost when TLS isn't in use at all.

## `rustls::ServerConfig`: `Arc`-shared across the worker pool

`tls_sanidi(cheti_njia, ufunguo_njia) -> Tokeo<TlsUsanidi, Neno>` loads a PEM certificate chain
and private key from disk (`rustls-pemfile` for parsing) and builds a `rustls::ServerConfig` via
`.with_no_client_auth().with_single_cert(certs, key)`. Every failure path — missing file,
malformed PEM, a cert/key that don't match — returns `Tokeo(Kosa(...))`, matching every other
constructor's error-handling convention in this codebase (`faili_fungua`, `mkondo_unganisha`);
`tls_sanidi` never panics on bad input (covered directly by
`tls_sanidi_missing_cert_file_returns_kosa_not_panic` and
`tls_sanidi_malformed_cert_returns_kosa_not_panic`).

The resulting config is wrapped as `Value::TlsUsanidi(Arc<rustls::ServerConfig>)`.
`rustls::ServerConfig` is `Send + Sync` by design — every rustls consumer `Arc`-shares it across
connections — so this fits `mkondo_tumikia`'s existing worker-pool model directly, the same way
`Arc<TcpListener>` (`MkondoSikilizaji`) is already shared across every worker thread. No new
concurrency primitive was needed.

## `mkondo_tumikia`'s 4th parameter, and the `Chaguo<T>` dual-representation trap

`mkondo_tumikia(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls: Chaguo<TlsUsanidi>)` — a 4th,
optional-by-`Chaguo` parameter rather than a separate `mkondo_tumikia_salama` function, since
every other part of the worker-loop contract (pool size, per-connection timeout, `kazi_jina`'s
`Mkondo`-handle contract) is identical whether or not TLS is active; only the accept-then-wrap
step branches.

This surfaced a real, non-obvious bug worth recording precisely, since it cost real debugging
time to find: **`Chaguo<T>` has two distinct runtime representations in this interpreter**, and
`mkondo_tumikia`'s Rust-side argument parsing initially only recognized one of them.
`eval/expr.rs`'s own `match_and_bind_pattern` already documents this for pattern-matching
(`linganisha`): a `Chaguo` can arrive as `Value::Chaguo(Some(_))` (produced by a builtin like a
cast or `kamusi.pata`) **or** as `Value::Enum("Chaguo", "Kuna", Some(_))` (produced by explicit
`Chaguo::Kuna(x)` construction syntax — the way an Asili caller actually writes
`mkondo_tumikia(s, kazi, n, Chaguo::Kuna(tls_sanidi(...)))`). A Rust-side function accepting a
`Chaguo`-typed argument by matching only `Value::Chaguo(Some(_))` silently treats a
`Chaguo::Kuna(...)`-constructed argument as absent — no error, the 4th argument just gets
ignored and TLS never activates, which manifested as the listener accepting a TLS handshake
attempt as if it were plaintext, hanging until `CONNECTION_TIMEOUT` expired. The fix: match both
shapes explicitly (see `mkondo_tumikia`'s `tls_inner`/`tls_config` construction in
`core/evaluator/src/builtins/mkondo.rs`). Any future builtin taking an optional `Chaguo<T>`
argument needs to do the same, unless the parser/evaluator later normalizes the two
representations into one canonical form.

## `send_close_notify` on drop — TLS's clean-close requirement

The second real bug this work surfaced: TLS requires a protocol-level `close_notify` message
before the underlying TCP socket closes. A bare TCP close — what `Wazi`'s teardown already gets
for free from `TcpStream`'s own `Drop` — is, by design, indistinguishable to a TLS peer from a
truncation attack, and `rustls` correctly errors ("peer closed connection without sending TLS
close_notify") rather than silently treating it as a clean EOF. Without an explicit fix, every
`.funga()`/scope-exit/drop of a TLS `Mkondo` handle would make the **peer's** next read fail even
though every application byte had already arrived correctly — this is exactly what happened
during this feature's own development: the request/response bytes were correct, but the test
client's final read errored anyway.

The fix: `MkondoStream` itself has a real `Drop` impl, sending `close_notify` and flushing for
the `Salama` case before the underlying `TcpStream` drops:

```rust
impl Drop for MkondoStream {
    fn drop(&mut self) {
        if let MkondoStream::Salama(s) = self {
            s.conn.send_close_notify();
            let _ = s.flush();
        }
    }
}
```

`Wazi` needs no equivalent — plain TCP has no such clean-close protocol requirement.

## Handshake-failure isolation

A TLS handshake failure (a plaintext client hitting a TLS-configured listener, a protocol
mismatch, garbage bytes) drops just that one connection — `worker_loop`'s TLS branch `continue`s
back to `accept()` rather than propagating the failure. This matches the same principle already
used for `kazi_jina` errors in the plaintext path: one bad connection must never take down a
whole worker thread and silently shrink the pool. Covered by
`plaintext_connection_to_tls_listener_fails_cleanly_not_by_hanging`.

The per-connection timeout (`apply_connection_timeout`, a fixed 30 seconds — see
`docs/design/faili-mkondo-design.md`) is applied to the raw `TcpStream` **before** any TLS
handshake attempt, not after — a stalled handshake needs the same protection a stalled plaintext
read does.

## Tests

`core/evaluator/src/builtins/mkondo.rs`'s own `#[cfg(test)]` module: `tls_sanidi`'s two error
paths (missing file, malformed PEM), calling the builtin function directly — no live socket
needed for these. `core/evaluator/tests/mkondo_tls.rs` (the role-flip pattern from
`mkondo_sikiliza.rs` — the Asili program is the server, a Rust-side client drives it):

- `tls_request_response_round_trips` — a full TLS 1.3 handshake, encrypted write, `close_notify`,
  and read, using a `rustls::RootCertStore` trusting exactly the one self-signed cert the test
  generates fresh via `rcgen` (a **dev-dependency only** — never shipped in the production
  dependency tree; no static cert/key pair is committed to the repo). This is the correct pattern
  for a private/self-issued CA, not a "disable verification" escape hatch.
- `plaintext_connection_to_tls_listener_fails_cleanly_not_by_hanging` — a plain `TcpStream`
  speaking no TLS at all against a TLS-configured listener; the connection closes instead of the
  server hanging forever waiting for a handshake that will never arrive.

Verified live, end-to-end, against the actual compiled `pata-cli` binary with a real minimal
Rust TLS client (not just the test harness) as the final proof this works outside of `cargo test`
too — the same verification standard applied to every prior phase's example
(`examples/mkondo_server/`).

## `wasm32` gating

`rustls`/`rustls-pemfile` are scoped to `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`
in `core/evaluator/Cargo.toml` — TLS is a native-listener-only capability (`mkondo_sikiliza`/
`mkondo_tumikia` are already fully gated out on `wasm32`, since there's no raw socket API in a
browser), and `rustls`'s crypto backend (`aws-lc-rs` by default) doesn't cross-compile cleanly to
`wasm32-unknown-unknown`. `Value::TlsUsanidi` and `MkondoStream::Salama` are both
`#[cfg(not(target_arch = "wasm32"))]`; `tls_sanidi` is registered only on non-`wasm32` targets.
Verified: `cargo build -p asili-wasm --target wasm32-unknown-unknown --features wasm-browser`
still builds clean with these changes in place.
