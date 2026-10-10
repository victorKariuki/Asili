//! Mkondo (network streams): `mkondo_unganisha` (connect: TCP, TLS or a Unix socket),
//! `mkondo_sikiliza` (listen), `tafuta_anwani` (DNS), `tls_sanidi` (a server certificate), the
//! methods of `Mkondo` and `MkondoSikilizaji`, and `mkondo_tumikia` (a pool of worker threads
//! serving each accepted connection to a named `kazi`). Connecting, listening and TLS are
//! `asili_mtandao`'s; every wait goes through `kazi_sawia::block_on`, so inside a `sawia` task
//! it lets the thread's other tasks run. A handle closes on `.funga()` or when dropped.

use std::collections::HashMap;

use super::BuiltinFn;
#[cfg(target_arch = "wasm32")]
use crate::value::{EvalError, Value};

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        m.insert("mkondo_unganisha".to_string(), Box::new(native::unganisha));
        m.insert("mkondo_sikiliza".to_string(), Box::new(native::sikiliza));
        m.insert("tafuta_anwani".to_string(), Box::new(native::tafuta_anwani));
        m.insert("tls_sanidi".to_string(), Box::new(native::tls_sanidi));
    }
    #[cfg(target_arch = "wasm32")]
    for name in ["mkondo_unganisha", "mkondo_sikiliza", "tafuta_anwani"] {
        m.insert(
            name.to_string(),
            Box::new(move |_: &[Value]| {
                Ok(Value::kosa(format!("{name}: haipatikani kwenye kivinjari")))
            }),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;

/// No sockets in the browser.
#[cfg(target_arch = "wasm32")]
pub(crate) fn mkondo_tumikia(
    _program: &crate::spawn::Shared,
    _args: &[Value],
) -> Result<Value, EvalError> {
    Ok(Value::kosa("mkondo_tumikia: haipatikani kwenye kivinjari"))
}

/// No sockets in the browser (a `Mkondo` cannot exist there).
#[cfg(target_arch = "wasm32")]
pub(crate) fn mkondo_method(
    cell: &std::rc::Rc<std::cell::RefCell<crate::value::MkondoHandle>>,
    _method: &str,
    _args: &[Value],
) -> Result<Value, EvalError> {
    match cell.borrow().0 {}
}

/// No sockets in the browser (a `MkondoSikilizaji` cannot exist there).
#[cfg(target_arch = "wasm32")]
pub(crate) fn sikilizaji_method(
    listener: &crate::value::Sikilizaji,
    _method: &str,
    _args: &[Value],
) -> Result<Value, EvalError> {
    match *listener {}
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::Duration;

    use asili_mtandao::{tls, BindOptions, ConnectOptions, LocalListener, Stream};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::super::http_thamani::field;
    use crate::kazi_sawia::block_on;
    use crate::value::{self, EvalError, MkondoHandle, Sikilizaji, Value};

    type Handle = Rc<RefCell<MkondoHandle>>;

    /// A wait's outcome as the program sees it: `Sawa(v)` or `Kosa(message)`.
    fn tokeo(r: Result<Value, String>) -> Value {
        match r {
            Ok(v) => Value::sawa(v),
            Err(e) => Value::kosa(e),
        }
    }

    fn text(v: &Value) -> Option<String> {
        match v {
            Value::Neno(s) => Some(s.to_string()),
            _ => None,
        }
    }

    fn seconds(v: &Value) -> Option<Duration> {
        value::as_f64(v)
            .filter(|s| *s > 0.0 && s.is_finite())
            .map(Duration::from_secs_f64)
    }

    fn handle(stream: Stream) -> Value {
        Value::Mkondo(Rc::new(RefCell::new(MkondoHandle::new(stream))))
    }

    /// `mkondo_unganisha(anwani, chaguo?: ChaguoMkondo) -> Tokeo<Mkondo, Neno>`.
    pub(super) fn unganisha(args: &[Value]) -> Result<Value, EvalError> {
        let addr = super::super::arg_str(args, 0);
        let o = args.get(1).unwrap_or(&Value::Hamna);
        let opts = ConnectOptions {
            tls: matches!(field(o, "tls"), Value::Ukweli(true)),
            server_name: text(field(o, "jina_seva")),
            timeout: seconds(field(o, "muda")),
            ca_file: text(field(o, "cheti_ca")),
            client_cert: text(field(o, "cheti")).zip(text(field(o, "ufunguo"))),
            ip_family: value::as_f64(field(o, "familia_ip")).map(|f| f as u8),
            alpn: Vec::new(),
        };
        crate::platform::flush_stdout(); // about to wait: show what was printed so far
        let r = block_on(async move { asili_mtandao::connect(&addr, &opts).await })?;
        Ok(tokeo(r.map(handle)))
    }

    /// `mkondo_sikiliza(anwani, chaguo?: ChaguoSikiliza) -> Tokeo<MkondoSikilizaji, Neno>`.
    pub(super) fn sikiliza(args: &[Value]) -> Result<Value, EvalError> {
        let addr = super::super::arg_str(args, 0);
        let o = args.get(1).unwrap_or(&Value::Hamna);
        let opts = BindOptions {
            reuse_port: matches!(field(o, "tumia_tena"), Value::Ukweli(true)),
            backlog: value::as_f64(field(o, "foleni")).map(|n| n.max(1.0) as i32),
        };
        Ok(tokeo(
            asili_mtandao::bind(&addr, &opts).map(|l| Value::MkondoSikilizaji(Arc::new(l))),
        ))
    }

    /// `tafuta_anwani(jina) -> Tokeo<Orodha<Neno>, Neno>`.
    pub(super) fn tafuta_anwani(args: &[Value]) -> Result<Value, EvalError> {
        let host = super::super::arg_str(args, 0);
        let r = block_on(async move { asili_mtandao::resolve(&host).await })?;
        Ok(tokeo(r.map(|ips| {
            Value::list(ips.into_iter().map(Value::neno).collect())
        })))
    }

    /// `tls_sanidi(cheti_njia, ufunguo_njia) -> Tokeo<TlsUsanidi, Neno>`: a certificate chain and
    /// its key (PEM files) for a TLS server. Every failure is a `Kosa`.
    pub(crate) fn tls_sanidi(args: &[Value]) -> Result<Value, EvalError> {
        let cert = super::super::arg_str(args, 0);
        let key = super::super::arg_str(args, 1);
        Ok(match tls::server_config(&cert, &key, &[]) {
            Ok(cfg) => Value::sawa(Value::TlsUsanidi(cfg)),
            Err(e) => Value::kosa(format!("tls_sanidi: {e}")),
        })
    }

    fn closed() -> String {
        "mkondo: imefungwa tayari".to_string()
    }

    /// `fut`, failing with "muda umekwisha" after `limit`.
    async fn within<T>(
        limit: Option<Duration>,
        fut: impl std::future::Future<Output = std::io::Result<T>>,
    ) -> Result<T, String> {
        match limit {
            Some(d) => match tokio::time::timeout(d, fut).await {
                Ok(r) => r.map_err(|e| e.to_string()),
                Err(_) => Err("mkondo: muda umekwisha".to_string()),
            },
            None => fut.await.map_err(|e| e.to_string()),
        }
    }

    /// Run one operation on the handle: it is borrowed for the whole wait, so two tasks cannot
    /// use one `Mkondo` at once (the second gets an error, not a mixed stream).
    async fn with_handle<T>(
        h: Handle,
        op: impl AsyncFnOnce(&mut MkondoHandle) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut g = h
            .try_borrow_mut()
            .map_err(|_| "mkondo: unatumiwa na kazi nyingine".to_string())?;
        op(&mut g).await
    }

    /// Up to `max` bytes: what was read ahead first, else one read (empty at the end).
    pub(crate) async fn read_some(h: &mut MkondoHandle, max: usize) -> Result<Vec<u8>, String> {
        if !h.buffered.is_empty() {
            let n = max.min(h.buffered.len());
            return Ok(h.buffered.drain(..n).collect());
        }
        let limit = h.timeout;
        let s = h.stream.as_mut().ok_or_else(closed)?;
        let mut buf = vec![0u8; max];
        let n = within(limit, s.read(&mut buf)).await?;
        buf.truncate(n);
        Ok(buf)
    }

    async fn read_to_end(h: &mut MkondoHandle) -> Result<Vec<u8>, String> {
        let limit = h.timeout;
        let mut out = std::mem::take(&mut h.buffered);
        let s = h.stream.as_mut().ok_or_else(closed)?;
        within(limit, s.read_to_end(&mut out)).await?;
        Ok(out)
    }

    async fn read_exact(h: &mut MkondoHandle, n: usize) -> Result<Vec<u8>, String> {
        let limit = h.timeout;
        let mut out: Vec<u8> = h.buffered.drain(..n.min(h.buffered.len())).collect();
        if out.len() < n {
            let s = h.stream.as_mut().ok_or_else(closed)?;
            let have = out.len();
            out.resize(n, 0);
            within(limit, s.read_exact(&mut out[have..]))
                .await
                .map_err(|e| format!("mkondo: baiti {n} hazikupatikana: {e}"))?;
        }
        Ok(out)
    }

    /// Longest line `.soma_mstari()` accepts.
    const MAX_LINE: usize = 1 << 20;

    /// The next line without its `\n` / `\r\n`; `None` at the end of the stream.
    async fn read_line(h: &mut MkondoHandle) -> Result<Option<Vec<u8>>, String> {
        let limit = h.timeout;
        let mut searched = 0;
        loop {
            if let Some(i) = h.buffered[searched..].iter().position(|b| *b == b'\n') {
                let mut line: Vec<u8> = h.buffered.drain(..=searched + i).collect();
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Ok(Some(line));
            }
            searched = h.buffered.len();
            if searched > MAX_LINE {
                return Err("mkondo: mstari mrefu mno".to_string());
            }
            let s = h.stream.as_mut().ok_or_else(closed)?;
            let mut chunk = [0u8; 8192];
            let n = within(limit, s.read(&mut chunk)).await?;
            if n == 0 {
                return Ok((!h.buffered.is_empty()).then(|| std::mem::take(&mut h.buffered)));
            }
            h.buffered.extend_from_slice(&chunk[..n]);
        }
    }

    pub(crate) async fn write_all(h: &mut MkondoHandle, data: &[u8]) -> Result<(), String> {
        let limit = h.timeout;
        let s = h.stream.as_mut().ok_or_else(closed)?;
        within(limit, async {
            s.write_all(data).await?;
            s.flush().await
        })
        .await
    }

    /// Wait for `op` on the handle `cell`, as a `Tokeo`.
    fn run<T: 'static>(
        cell: &Handle,
        op: impl AsyncFnOnce(&mut MkondoHandle) -> Result<T, String> + 'static,
        out: impl FnOnce(T) -> Value,
    ) -> Result<Value, EvalError> {
        crate::platform::flush_stdout(); // about to wait: show what was printed so far
        let r = block_on(with_handle(cell.clone(), op))?;
        Ok(tokeo(r.map(out)))
    }

    fn count(args: &[Value]) -> usize {
        value::as_f64(args.first().unwrap_or(&Value::Hamna))
            .unwrap_or(0.0)
            .max(0.0) as usize
    }

    /// A method of `Mkondo`.
    pub(crate) fn mkondo_method(
        cell: &Handle,
        method: &str,
        args: &[Value],
    ) -> Result<Value, EvalError> {
        match method {
            "soma" => run(cell, read_to_end, |b| match String::from_utf8(b) {
                Ok(s) => Value::neno(s),
                Err(e) => Value::neno(String::from_utf8_lossy(e.as_bytes()).into_owned()),
            }),
            "soma_baiti" => {
                let max = count(args);
                run(
                    cell,
                    async move |h| read_some(h, max).await,
                    |b| Value::Baiti(b.into()),
                )
            }
            // The same read, as text (invalid UTF-8 replaced).
            "soma_bailisi" => {
                let max = count(args);
                run(
                    cell,
                    async move |h| read_some(h, max).await,
                    |b| Value::neno(String::from_utf8_lossy(&b).into_owned()),
                )
            }
            "soma_kamili" => {
                let n = count(args);
                run(
                    cell,
                    async move |h| read_exact(h, n).await,
                    |b| Value::Baiti(b.into()),
                )
            }
            "soma_mstari" => run(cell, read_line, |line| {
                Value::Chaguo(
                    line.map(|l| Box::new(Value::neno(String::from_utf8_lossy(&l).into_owned()))),
                )
            }),
            "andika" => {
                let data = crate::eval::methods::bytes_of(args.first().unwrap_or(&Value::Hamna))
                    .map(|b| b.into_owned())
                    .unwrap_or_default();
                run(
                    cell,
                    async move |h| write_all(h, &data).await,
                    |()| Value::Tupu,
                )
            }
            "funga_kuandika" => run(
                cell,
                async |h| {
                    let limit = h.timeout;
                    let s = h.stream.as_mut().ok_or_else(closed)?;
                    within(limit, s.shutdown()).await
                },
                |()| Value::Tupu,
            ),
            "funga" => {
                let stream = cell.try_borrow_mut().ok().and_then(|mut h| h.stream.take());
                if let Some(mut s) = stream {
                    // Closing waits (briefly) for TLS's goodbye; the socket closes either way.
                    let _ = block_on(async move {
                        let _ = tokio::time::timeout(Duration::from_secs(5), s.shutdown()).await;
                    });
                }
                Ok(Value::Tupu)
            }
            "anwani_mbali" | "anwani_yangu" => {
                let h = cell.borrow();
                Ok(Value::neno(match h.stream.as_ref() {
                    Some(s) if method == "anwani_mbali" => s.peer_addr(),
                    Some(s) => s.local_addr(),
                    None => String::new(),
                }))
            }
            "weka_muda" => {
                cell.borrow_mut().timeout = seconds(args.first().unwrap_or(&Value::Hamna));
                Ok(Value::Tupu)
            }
            _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
        }
    }

    /// A method of `MkondoSikilizaji`.
    pub(crate) fn sikilizaji_method(
        listener: &Arc<Sikilizaji>,
        method: &str,
        _args: &[Value],
    ) -> Result<Value, EvalError> {
        match method {
            "kubali" => {
                crate::platform::flush_stdout();
                let l = listener.clone();
                let r = block_on(async move {
                    match l.local()?.accept().await? {
                        Some((s, _)) => Ok(handle(s)),
                        None => Err("mkondo: msikilizaji amesimamishwa".to_string()),
                    }
                })?;
                Ok(tokeo(r))
            }
            "anwani" => Ok(Value::neno(listener.local_addr())),
            "simama" => {
                listener.stop();
                Ok(Value::Tupu)
            }
            _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
        }
    }

    /// Time limit of each read and write on a server's connection: a stalled or hostile peer
    /// must not hold a worker forever. Shared with `http.rs`.
    pub(crate) const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

    /// `mkondo_tumikia(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls?) -> Tokeo<Tupu, Neno>`: serve
    /// every connection to `kazi_jina(mkondo: Mkondo)` on `idadi_ya_nyuzi` worker threads, until
    /// the listener is stopped (`.simama()`).
    pub(crate) fn mkondo_tumikia(
        program: &crate::spawn::Shared,
        args: &[Value],
    ) -> Result<Value, EvalError> {
        serve_pool("mkondo_tumikia", program, args, |call, kazi, mkondo| {
            // One failing connection must not end the worker (and shrink the pool).
            let _ = call(kazi, vec![mkondo]);
        })
    }

    /// A worker's handling of one connection: `(caller, kazi_jina, mkondo)`.
    pub(super) type Serve = fn(&mut crate::spawn::Caller<'_>, &str, Value);

    /// The worker pool shared by `mkondo_tumikia` and `mkondo_tumikia_http`: check
    /// `(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls?)`, then on that many threads accept
    /// connections (completing TLS when given), hand each to `serve`, and return once the
    /// listener is stopped. Each worker builds its engine for `program` once.
    pub(crate) fn serve_pool(
        name: &str,
        program: &crate::spawn::Shared,
        args: &[Value],
        serve: Serve,
    ) -> Result<Value, EvalError> {
        let listener = match args.first() {
            Some(Value::MkondoSikilizaji(l)) => Arc::clone(l),
            _ => {
                return Ok(Value::kosa(format!(
                    "{name}: hoja ya kwanza lazima iwe MkondoSikilizaji"
                )))
            }
        };
        let kazi_name = super::super::arg_str(args, 1);
        if !program.has_kazi(&kazi_name) {
            return Ok(Value::kosa(format!(
                "{name}: kazi haijulikani: {kazi_name}"
            )));
        }
        let threads = value::as_f64(args.get(2).unwrap_or(&Value::Hamna))
            .unwrap_or(0.0)
            .max(1.0) as usize;
        let tls_config =
            match super::super::http_thamani::unwrap_some(args.get(3).unwrap_or(&Value::Hamna)) {
                Value::Hamna => None,
                Value::TlsUsanidi(cfg) => Some(Arc::clone(cfg)),
                _ => {
                    return Ok(Value::kosa(format!(
                        "{name}: hoja ya nne (tls) lazima iwe Chaguo<TlsUsanidi>"
                    )))
                }
            };
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let listener = Arc::clone(&listener);
                let program = program.clone();
                let kazi_name = kazi_name.clone();
                let tls_config = tls_config.clone();
                std::thread::spawn(move || {
                    program
                        .with_caller(|call| worker(&listener, call, &kazi_name, tls_config, serve))
                })
            })
            .collect();
        for h in handles {
            let _ = h.join();
        }
        Ok(Value::sawa(Value::Tupu))
    }

    fn worker(
        listener: &Arc<Sikilizaji>,
        call: &mut crate::spawn::Caller<'_>,
        kazi_name: &str,
        tls_config: Option<Arc<rustls::ServerConfig>>,
        serve: Serve,
    ) {
        let l = listener.clone();
        let Ok(Ok(local)) = block_on(async move { l.local() }) else {
            return;
        };
        let local = Rc::new(local);
        loop {
            crate::platform::flush_stdout();
            let (local, tls_config) = (local.clone(), tls_config.clone());
            let accepted = block_on(async move { accept(&local, tls_config).await });
            match accepted {
                Ok(Ok(Some(stream))) => {
                    let mut h = MkondoHandle::new(stream);
                    h.timeout = Some(CONNECTION_TIMEOUT);
                    serve(call, kazi_name, Value::Mkondo(Rc::new(RefCell::new(h))));
                }
                // A failed accept or handshake drops that one connection; a pause keeps a
                // persistent failure (no file descriptors left) from spinning.
                Ok(Err(_)) => {
                    let _ = crate::kazi_sawia::lala(0.01);
                }
                Ok(Ok(None)) | Err(_) => return,
            }
        }
    }

    /// The next connection, with TLS completed when configured; `None` once stopped.
    async fn accept(
        local: &LocalListener,
        tls_config: Option<Arc<rustls::ServerConfig>>,
    ) -> Result<Option<Stream>, String> {
        let Some((stream, _)) = local.accept().await? else {
            return Ok(None);
        };
        match tls_config {
            Some(cfg) => tokio::time::timeout(CONNECTION_TIMEOUT, tls::accept(cfg, stream))
                .await
                .map_err(|_| "TLS: muda umekwisha".to_string())?
                .map(Some),
            None => Ok(Some(stream)),
        }
    }

    /// `std::io` reads and writes on a connection, for the HTTP server's parser.
    pub(crate) struct Blocking(pub(crate) Handle);

    impl std::io::Read for Blocking {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let max = buf.len();
            let data = block_on(with_handle(self.0.clone(), async move |h| {
                read_some(h, max).await
            }))
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?
            .map_err(|e| {
                let kind = if e.ends_with("muda umekwisha") {
                    std::io::ErrorKind::TimedOut
                } else {
                    std::io::ErrorKind::Other
                };
                std::io::Error::new(kind, e)
            })?;
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    impl std::io::Write for Blocking {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let data = buf.to_vec();
            block_on(with_handle(self.0.clone(), async move |h| {
                write_all(h, &data).await
            }))
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?
            .map_err(std::io::Error::other)?;
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn tls_sanidi_missing_or_malformed_files_are_kosa() {
            let result = tls_sanidi(&[
                Value::neno("/nonexistent/path/cert.pem".to_string()),
                Value::neno("/nonexistent/path/key.pem".to_string()),
            ]);
            assert!(matches!(result, Ok(Value::Tokeo(Err(_)))), "{result:?}");

            let dir = std::env::temp_dir();
            let cert = dir.join(format!("asili-bad-cert-{}.pem", std::process::id()));
            let key = dir.join(format!("asili-bad-key-{}.pem", std::process::id()));
            std::fs::write(&cert, "hii sio PEM halali").unwrap();
            std::fs::write(&key, "hii sio PEM halali pia").unwrap();
            let result = tls_sanidi(&[
                Value::neno(cert.to_string_lossy().to_string()),
                Value::neno(key.to_string_lossy().to_string()),
            ]);
            assert!(matches!(result, Ok(Value::Tokeo(Err(_)))), "{result:?}");
            let _ = std::fs::remove_file(&cert);
            let _ = std::fs::remove_file(&key);
        }
    }
}
