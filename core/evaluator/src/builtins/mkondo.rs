//! Mkondo (network stream/socket): mkondo_unganisha (connect a TCP client stream),
//! mkondo_sikiliza (bind a listening socket), mkondo_tumikia (bounded worker-thread pool
//! serving accepted connections to a named kazi). Returns Tokeo<Mkondo, Neno>/
//! Tokeo<MkondoSikilizaji, Neno>. Handles close automatically on drop (scope exit, explicit
//! tupa, or .funga()) via MkondoHandle's own Drop impl — see docs/design/faili-mkondo-design.md
//! and docs/design/http-server-design.md for the bounded-pool-over-async decision.

use std::cell::RefCell;
use std::collections::HashMap;
use std::net::TcpStream;
use std::rc::Rc;
use std::sync::Arc;

use crate::value::{self, MkondoHandle, MkondoStream, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("mkondo_unganisha".to_string(), Box::new(|args: &[Value]| {
        let addr = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            match TcpStream::connect(&addr) {
                Ok(s) => {
                    let handle = MkondoHandle(Some(MkondoStream::Wazi(s)));
                    Ok(Value::Tokeo(Ok(Box::new(Value::Mkondo(Rc::new(RefCell::new(handle)))))))
                }
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "mkondo_unganisha: haipatikani kwenye kivinjari".into(),
        )))))
    }));
    m.insert("mkondo_sikiliza".to_string(), Box::new(|args: &[Value]| {
        let addr = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            match std::net::TcpListener::bind(&addr) {
                Ok(l) => Ok(Value::Tokeo(Ok(Box::new(Value::MkondoSikilizaji(Arc::new(l)))))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "mkondo_sikiliza: haipatikani kwenye kivinjari".into(),
        )))))
    }));
    #[cfg(not(target_arch = "wasm32"))]
    m.insert("tls_sanidi".to_string(), Box::new(tls_sanidi));
}

/// `tls_sanidi(cheti_njia: Neno, ufunguo_njia: Neno) -> Tokeo<TlsUsanidi, Neno>` — loads a PEM
/// certificate chain and private key from disk, building a `rustls::ServerConfig` ready to hand
/// to `mkondo_tumikia`'s optional `tls` parameter. Never panics on a missing/malformed
/// file or a key/cert mismatch — every failure surfaces as `Tokeo(Kosa(...))`, matching every
/// other constructor's error-handling convention in this codebase (`faili_fungua`,
/// `mkondo_unganisha`, etc.).
#[cfg(not(target_arch = "wasm32"))]
fn tls_sanidi(args: &[Value]) -> Result<Value, value::EvalError> {
    let cheti_njia = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
    let ufunguo_njia = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();

    let cheti_bytes = match std::fs::read(&cheti_njia) {
        Ok(b) => b,
        Err(e) => return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!("tls_sanidi: {e}")))))),
    };
    let ufunguo_bytes = match std::fs::read(&ufunguo_njia) {
        Ok(b) => b,
        Err(e) => return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!("tls_sanidi: {e}")))))),
    };

    let certs: Result<Vec<_>, _> = rustls_pemfile::certs(&mut cheti_bytes.as_slice()).collect();
    let certs = match certs {
        Ok(c) if !c.is_empty() => c,
        Ok(_) => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "tls_sanidi: hakuna cheti kwenye faili".to_string(),
            )))))
        }
        Err(e) => return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!("tls_sanidi: cheti batili: {e}")))))),
    };
    let key = match rustls_pemfile::private_key(&mut ufunguo_bytes.as_slice()) {
        Ok(Some(k)) => k,
        Ok(None) => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "tls_sanidi: hakuna ufunguo kwenye faili".to_string(),
            )))))
        }
        Err(e) => return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!("tls_sanidi: ufunguo batili: {e}")))))),
    };

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key);
    match config {
        Ok(cfg) => Ok(Value::Tokeo(Ok(Box::new(Value::TlsUsanidi(Arc::new(cfg)))))),
        Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
            "tls_sanidi: cheti na ufunguo havilingani: {e}"
        )))))),
    }
}

/// Per-connection I/O timeout: an unbounded blocking read/write on a stalled or malicious peer
/// would otherwise permanently occupy one of the pool's fixed worker threads, degrading the
/// whole pool over time — this is not optional for a production listener (see the original
/// production-readiness survey). Fixed rather than configurable for this pass; a configurable
/// timeout is a natural follow-up once real usage data exists. `pub(crate)`: shared with
/// `http.rs`'s `mkondo_tumikia_http`, which needs the identical per-connection timeout
/// behavior — not redeclared there.
pub(crate) const CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// `mkondo_tumikia(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls: Chaguo<TlsUsanidi>) ->
/// Tokeo<Tupu, Neno>`: spawns `idadi_ya_nyuzi` long-lived worker threads, each independently
/// looping accept -> (optional TLS handshake) -> invoke `kazi_jina(mkondo: Mkondo) -> Tupu` via
/// the same `run_function`-per-call pattern `tenda` already established -> repeat. Blocks the
/// calling thread forever (joins every worker), matching a simple "kazi kuu's last statement
/// starts the server" shape — there is no separate stop/handle mechanism in this pass.
///
/// The accepted `std::net::TcpStream` is constructed and consumed entirely inside the worker
/// thread that accepted it — it never crosses a `SendValue` boundary, unlike `tenda`'s spawned
/// function's arguments. This is what a fixed-size pool of N long-lived threads sharing one
/// `Arc<TcpListener>` buys over `tenda`-per-connection: the worker's `kazi_jina` receives a real,
/// live `Mkondo` handle it can `.soma()`/`.andika()` on directly, exactly like
/// `mkondo_unganisha`'s client-side handle today.
///
/// `tls` is a 4th, optional-by-`Chaguo` parameter rather than a separate
/// `mkondo_tumikia_salama` function: every other part of the worker-loop contract (pool size,
/// per-connection timeout, `kazi_jina`'s `Mkondo`-handle contract) is identical whether or not
/// TLS is active, so only the accept-then-wrap step branches.
///
/// Not registered as an ordinary `BuiltinFn` — like `tenda`, it needs the current `Module` to
/// find `kazi_jina`, so it's special-cased in `eval/expr.rs`'s `Expr::Call` handling.
pub(crate) fn mkondo_tumikia(
    module: &asili_parser::Module,
    args: &[Value],
) -> Result<Value, value::EvalError> {
    let listener = match args.first() {
        Some(Value::MkondoSikilizaji(l)) => Arc::clone(l),
        _ => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "mkondo_tumikia: hoja ya kwanza lazima iwe MkondoSikilizaji".to_string(),
            )))))
        }
    };
    let kazi_name = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
    if !module.functions.iter().any(|f| f.name == kazi_name) {
        return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
            "mkondo_tumikia: kazi haijulikani: {kazi_name}"
        ))))));
    }
    let idadi_ya_nyuzi = value::as_f64(args.get(2).unwrap_or(&Value::Hamna))
        .unwrap_or(0.0)
        .max(1.0) as usize;
    // `Chaguo<T>` has two runtime shapes here, same as everywhere else in this codebase (see
    // eval/expr.rs's match_and_bind_pattern comment on Tokeo/Chaguo): `Value::Chaguo(Some(_))`
    // (from a builtin/cast producing one) and `Value::Enum("Chaguo", "Kuna", Some(_))` (from an
    // explicit `Chaguo::Kuna(tls_sanidi(...))` construction, which is how an Asili caller
    // actually writes this argument) — both must be accepted, or the explicit-construction path
    // silently falls through to "no TLS" instead of an error.
    #[cfg(not(target_arch = "wasm32"))]
    let tls_inner: Option<&Value> = match args.get(3) {
        Some(Value::Chaguo(Some(inner))) => Some(inner.as_ref()),
        Some(Value::Enum(en, vn, Some(inner))) if en == "Chaguo" && vn == "Kuna" => Some(inner.as_ref()),
        Some(Value::Chaguo(None)) => None,
        Some(Value::Enum(en, vn, None)) if en == "Chaguo" && vn == "Hamna" => None,
        None | Some(Value::Hamna) => None,
        Some(_) => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "mkondo_tumikia: hoja ya nne (tls) lazima iwe Chaguo<TlsUsanidi>".to_string(),
            )))))
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    let tls_config: Option<Arc<rustls::ServerConfig>> = match tls_inner {
        Some(Value::TlsUsanidi(cfg)) => Some(Arc::clone(cfg)),
        Some(_) => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "mkondo_tumikia: hoja ya nne (tls) lazima iwe Chaguo<TlsUsanidi>".to_string(),
            )))))
        }
        None => None,
    };

    let mut handles = Vec::with_capacity(idadi_ya_nyuzi);
    for _ in 0..idadi_ya_nyuzi {
        let listener = Arc::clone(&listener);
        let module_owned = module.clone();
        let kazi_name = kazi_name.clone();
        #[cfg(not(target_arch = "wasm32"))]
        let tls_config = tls_config.clone();
        handles.push(std::thread::spawn(move || {
            #[cfg(not(target_arch = "wasm32"))]
            worker_loop(&listener, &module_owned, &kazi_name, tls_config);
            #[cfg(target_arch = "wasm32")]
            worker_loop(&listener, &module_owned, &kazi_name);
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    Ok(Value::Tokeo(Ok(Box::new(Value::Tupu))))
}

/// `kazi_jina`'s contract: `kazi_jina(mkondo: Mkondo) -> Tupu` — it owns the whole connection
/// lifecycle itself via the same `.soma()`/`.andika()`/`.funga()` methods `mkondo_unganisha`'s
/// client-side handle already exposes (read the request, write a response, done). The handle
/// closes automatically via `MkondoHandle`'s `Drop` impl once `run_function` returns, whether
/// `kazi_jina` called `.funga()` explicitly or not — identical to every other `Mkondo`/`Faili`
/// teardown path in this codebase. Transparent to whether the connection is plain or TLS — both
/// are just `Mkondo` handles wrapping a `MkondoStream`.
fn worker_loop(
    listener: &std::net::TcpListener,
    module: &asili_parser::Module,
    kazi_name: &str,
    #[cfg(not(target_arch = "wasm32"))] tls_config: Option<Arc<rustls::ServerConfig>>,
) {
    loop {
        let stream = match listener.accept() {
            Ok((s, _addr)) => s,
            // The listener itself closing (every Arc clone dropped, or a real bind error) ends
            // this worker's loop rather than spinning on a permanently-broken accept.
            Err(_) => return,
        };
        // The timeout applies to the raw TCP socket before any TLS handshake — a stalled
        // handshake needs the same protection a stalled plaintext read does.
        apply_connection_timeout(&stream);

        #[cfg(not(target_arch = "wasm32"))]
        let mkondo_stream = match &tls_config {
            Some(cfg) => match handshake_tls(Arc::clone(cfg), stream) {
                Some(s) => s,
                // A handshake failure (bad client, protocol mismatch, garbage bytes) drops just
                // this one connection — it must not be treated as a fatal error for the whole
                // worker thread, matching the "one connection's failure doesn't take down the
                // worker" principle already used for kazi_jina errors below.
                None => continue,
            },
            None => MkondoStream::Wazi(stream),
        };
        #[cfg(target_arch = "wasm32")]
        let mkondo_stream = MkondoStream::Wazi(stream);

        let mkondo = Value::Mkondo(Rc::new(RefCell::new(MkondoHandle(Some(mkondo_stream)))));
        // A panic or error inside kazi_jina for one connection must not take down this worker
        // thread (and silently shrink the pool) — log-and-continue is the only reasonable
        // behavior for a long-lived server loop; there's no caller left to propagate the error
        // to once we're this deep inside a spawned worker thread.
        let _ = crate::run_function(module, kazi_name, vec![mkondo]);
    }
}

/// `pub(crate)`: shared with `http.rs`'s `mkondo_tumikia_http`, which performs the identical
/// TLS handshake step on an accepted connection — not reimplemented there.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn handshake_tls(config: Arc<rustls::ServerConfig>, stream: TcpStream) -> Option<MkondoStream> {
    let conn = rustls::ServerConnection::new(config).ok()?;
    Some(MkondoStream::Salama(Box::new(rustls::StreamOwned::new(conn, stream))))
}

/// Extracted so the timeout value actually applied to an accepted socket is independently
/// unit-testable (`tests::connection_timeout_is_set_on_accept`) without needing a live,
/// real-time 30-second stall to observe the effect end-to-end. `pub(crate)`: shared with
/// `http.rs`.
pub(crate) fn apply_connection_timeout(stream: &TcpStream) {
    let _ = stream.set_read_timeout(Some(CONNECTION_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CONNECTION_TIMEOUT));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_timeout_is_set_on_accept() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            apply_connection_timeout(&stream);
            (stream.read_timeout().expect("read_timeout"), stream.write_timeout().expect("write_timeout"))
        });
        let _client = TcpStream::connect(addr).expect("connect");
        let (read_timeout, write_timeout) = server.join().expect("server thread");
        assert_eq!(read_timeout, Some(CONNECTION_TIMEOUT));
        assert_eq!(write_timeout, Some(CONNECTION_TIMEOUT));
    }

    #[test]
    fn tls_sanidi_missing_cert_file_returns_kosa_not_panic() {
        let result = tls_sanidi(&[
            Value::Neno("/nonexistent/path/cert.pem".to_string()),
            Value::Neno("/nonexistent/path/key.pem".to_string()),
        ]);
        match result {
            Ok(Value::Tokeo(Err(_))) => {}
            other => panic!("expected Tokeo(Kosa(...)), got {other:?}"),
        }
    }

    #[test]
    fn tls_sanidi_malformed_cert_returns_kosa_not_panic() {
        let dir = std::env::temp_dir();
        let cert_path = dir.join(format!("asili-bad-cert-{:?}.pem", std::thread::current().id()));
        let key_path = dir.join(format!("asili-bad-key-{:?}.pem", std::thread::current().id()));
        std::fs::write(&cert_path, "hii sio PEM halali").unwrap();
        std::fs::write(&key_path, "hii sio PEM halali pia").unwrap();

        let result = tls_sanidi(&[
            Value::Neno(cert_path.to_string_lossy().to_string()),
            Value::Neno(key_path.to_string_lossy().to_string()),
        ]);
        match result {
            Ok(Value::Tokeo(Err(_))) => {}
            other => panic!("expected Tokeo(Kosa(...)), got {other:?}"),
        }

        let _ = std::fs::remove_file(&cert_path);
        let _ = std::fs::remove_file(&key_path);
    }
}
