//! Mkondo (network streams): `mkondo_unganisha` (connect: TCP, TLS or a Unix socket),
//! `mkondo_sikiliza` (listen), `tafuta_anwani` (DNS), `tls_sanidi` (a server certificate), the
//! methods of `Mkondo` and `MkondoSikilizaji`, and `mkondo_tumikia` (a pool of worker threads
//! serving each accepted connection to a named `kazi`). Connecting, listening and TLS are
//! `asili_mtandao`'s; every wait goes through `kazi_sawia::block_on`, so inside a `sawia` task
//! it lets the thread's other tasks run. A handle closes on `.funga()` or when dropped.

use std::collections::HashMap;

use super::BuiltinFn;
#[cfg(target_arch = "wasm32")]
use crate::value::EvalError;
use crate::value::Value;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "tukio_sse".to_string(),
        Box::new(|args: &[Value]| {
            let opt = |i: usize| {
                crate::value::as_string(super::http_thamani::unwrap_some(
                    args.get(i).unwrap_or(&Value::Hamna),
                ))
            };
            let data = super::arg_str(args, 0);
            Ok(Value::neno(super::http_thamani::sse_event(
                &data,
                opt(1).as_deref(),
                opt(2).as_deref(),
            )))
        }),
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        m.insert("mkondo_unganisha".to_string(), Box::new(native::unganisha));
        m.insert("mkondo_sikiliza".to_string(), Box::new(native::sikiliza));
        m.insert("tafuta_anwani".to_string(), Box::new(native::tafuta_anwani));
        m.insert("tls_sanidi".to_string(), Box::new(native::tls_sanidi));
        m.insert("udp_fungua".to_string(), Box::new(native::udp_fungua));
        m.insert("ws_unganisha".to_string(), Box::new(native::ws_unganisha));
    }
    #[cfg(target_arch = "wasm32")]
    for name in [
        "mkondo_unganisha",
        "mkondo_sikiliza",
        "tafuta_anwani",
        "udp_fungua",
        "ws_unganisha",
    ] {
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

/// No sockets in the browser (a `MkondoWs` cannot exist there).
#[cfg(target_arch = "wasm32")]
pub(crate) fn ws_method(
    w: &std::rc::Rc<std::cell::RefCell<crate::value::WsHandle>>,
    _method: &str,
    _args: &[Value],
) -> Result<Value, EvalError> {
    match w.borrow().0 {}
}

/// No sockets in the browser (a `MkondoUdp` cannot exist there).
#[cfg(target_arch = "wasm32")]
pub(crate) fn udp_method(
    u: &crate::value::MkondoUdp,
    _method: &str,
    _args: &[Value],
) -> Result<Value, EvalError> {
    match u.0 {}
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
    use crate::value::{self, EvalError, MkondoHandle, Sikilizaji, Value, WsHandle};

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

    /// `ws_unganisha(anwani, chaguo?: ChaguoMkondo) -> Tokeo<MkondoWs, Neno>`: `ws://` or `wss://`.
    pub(super) fn ws_unganisha(args: &[Value]) -> Result<Value, EvalError> {
        let addr = super::super::arg_str(args, 0);
        let o = args.get(1).unwrap_or(&Value::Hamna);
        let opts = ConnectOptions {
            tls: false,
            server_name: text(field(o, "jina_seva")),
            timeout: seconds(field(o, "muda")),
            ca_file: text(field(o, "cheti_ca")),
            client_cert: text(field(o, "cheti")).zip(text(field(o, "ufunguo"))),
            ip_family: value::as_f64(field(o, "familia_ip")).map(|f| f as u8),
            alpn: Vec::new(),
        };
        crate::platform::flush_stdout(); // about to wait: show what was printed so far
        let r = block_on(async move { asili_mtandao::ws::connect(&addr, &opts).await })?;
        Ok(tokeo(r.map(|ws| {
            Value::MkondoWs(Rc::new(RefCell::new(WsHandle(Some(ws)))))
        })))
    }

    /// Wait for `op` on the open WebSocket `w`, as a `Tokeo`.
    fn run_ws<T: 'static>(
        w: &Rc<RefCell<WsHandle>>,
        op: impl AsyncFnOnce(&mut asili_mtandao::ws::Ws) -> Result<T, String> + 'static,
        out: impl FnOnce(T) -> Value,
    ) -> Result<Value, EvalError> {
        crate::platform::flush_stdout(); // about to wait: show what was printed so far
        let w = w.clone();
        let r = block_on(async move {
            let mut g = w
                .try_borrow_mut()
                .map_err(|_| "WebSocket: inatumiwa na kazi nyingine".to_string())?;
            let ws =
                g.0.as_mut()
                    .ok_or_else(|| "WebSocket: imefungwa tayari".to_string())?;
            op(ws).await
        })?;
        Ok(tokeo(r.map(out)))
    }

    /// A method of `MkondoWs`.
    pub(crate) fn ws_method(
        w: &Rc<RefCell<WsHandle>>,
        method: &str,
        args: &[Value],
    ) -> Result<Value, EvalError> {
        use asili_mtandao::ws::{self, Message};
        let unit = |()| Value::Tupu;
        let payload = |m: Message| match m {
            Message::Binary(b) => b.to_vec(),
            Message::Text(t) => t.as_bytes().to_vec(),
            _ => Vec::new(),
        };
        match method {
            "tuma" => {
                let text = super::super::arg_str(args, 0);
                run_ws(
                    w,
                    async move |ws| ws::send(ws, Message::Text(text.into())).await,
                    unit,
                )
            }
            "tuma_baiti" => {
                let data = crate::eval::methods::bytes_of(args.first().unwrap_or(&Value::Hamna))
                    .map(|b| b.into_owned())
                    .unwrap_or_default();
                run_ws(
                    w,
                    async move |ws| ws::send(ws, Message::Binary(data.into())).await,
                    unit,
                )
            }
            "pokea" => run_ws(
                w,
                async |ws| ws::recv(ws, None).await,
                move |m| Value::Chaguo(m.map(|m| Box::new(Value::Baiti(payload(m).into())))),
            ),
            "pokea_neno" => run_ws(
                w,
                async |ws| ws::recv(ws, None).await,
                move |m| {
                    Value::Chaguo(m.map(|m| {
                        Box::new(Value::neno(
                            String::from_utf8_lossy(&payload(m)).into_owned(),
                        ))
                    }))
                },
            ),
            "funga" => {
                let taken = w.try_borrow_mut().ok().and_then(|mut h| h.0.take());
                if let Some(mut ws) = taken {
                    block_on(async move { ws::close(&mut ws).await })?;
                }
                Ok(Value::sawa(Value::Tupu))
            }
            _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
        }
    }

    /// `udp_fungua(anwani) -> Tokeo<MkondoUdp, Neno>`.
    pub(super) fn udp_fungua(args: &[Value]) -> Result<Value, EvalError> {
        let addr = super::super::arg_str(args, 0);
        let r = block_on(async move { asili_mtandao::udp_bind(&addr).await })?;
        Ok(tokeo(r.map(|udp| {
            Value::MkondoUdp(Rc::new(value::MkondoUdp {
                udp,
                timeout: std::cell::Cell::new(None),
            }))
        })))
    }

    fn payload(args: &[Value], i: usize) -> Vec<u8> {
        crate::eval::methods::bytes_of(args.get(i).unwrap_or(&Value::Hamna))
            .map(|b| b.into_owned())
            .unwrap_or_default()
    }

    /// Wait for `op` on the UDP socket `u`, as a `Tokeo`.
    fn run_udp<T: 'static>(
        u: &Rc<value::MkondoUdp>,
        op: impl AsyncFnOnce(&value::MkondoUdp) -> Result<T, String> + 'static,
        out: impl FnOnce(T) -> Value,
    ) -> Result<Value, EvalError> {
        crate::platform::flush_stdout(); // about to wait: show what was printed so far
        let u = u.clone();
        let r = block_on(async move { op(&u).await })?;
        Ok(tokeo(r.map(out)))
    }

    /// A method of `MkondoUdp`.
    pub(crate) fn udp_method(
        u: &Rc<value::MkondoUdp>,
        method: &str,
        args: &[Value],
    ) -> Result<Value, EvalError> {
        let unit = |()| Value::Tupu;
        match method {
            "tuma_kwa" => {
                let (to, data) = (super::super::arg_str(args, 0), payload(args, 1));
                run_udp(u, async move |u| u.udp.send_to(&data, &to).await, unit)
            }
            "tuma" => {
                let data = payload(args, 0);
                run_udp(u, async move |u| u.udp.send(&data).await, unit)
            }
            "unganisha" => {
                let to = super::super::arg_str(args, 0);
                run_udp(u, async move |u| u.udp.connect(&to).await, unit)
            }
            "pokea" => {
                let max = match args.first() {
                    Some(v) => value::as_f64(v).unwrap_or(0.0).max(1.0) as usize,
                    None => 65536,
                };
                run_udp(
                    u,
                    async move |u| {
                        let limit = u.timeout.get();
                        match limit {
                            Some(d) => tokio::time::timeout(d, u.udp.recv_from(max))
                                .await
                                .map_err(|_| "udp: muda umekwisha".to_string())?,
                            None => u.udp.recv_from(max).await,
                        }
                    },
                    |(data, from)| {
                        Value::Jozi(
                            Box::new(Value::Baiti(data.into())),
                            Box::new(Value::neno(from)),
                        )
                    },
                )
            }
            "anwani" => Ok(Value::neno(u.udp.local_addr())),
            "weka_muda" => {
                u.timeout
                    .set(seconds(args.first().unwrap_or(&Value::Hamna)));
                Ok(Value::Tupu)
            }
            "tangaza" => {
                let on = !matches!(args.first(), Some(Value::Ukweli(false)));
                Ok(tokeo(u.udp.set_broadcast(on).map(unit)))
            }
            "jiunge_kikundi" => Ok(tokeo(
                u.udp
                    .join_multicast(&super::super::arg_str(args, 0))
                    .map(unit),
            )),
            _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
        }
    }

    /// Time limit of each read and write on a server's connection: a stalled or hostile peer
    /// must not hold a connection forever.
    pub(crate) const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

    /// How long a stopped `mkondo_tumikia`'s open connections may take to finish.
    const GRACE: Duration = Duration::from_secs(10);

    /// `mkondo_tumikia(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls?) -> Tokeo<Tupu, Neno>`: serve
    /// every connection to `kazi_jina(mkondo: Mkondo)`, each as a `sawia` task, on
    /// `idadi_ya_nyuzi` worker threads, until the listener is stopped (`.simama()`).
    pub(crate) fn mkondo_tumikia(
        program: &crate::spawn::Shared,
        args: &[Value],
    ) -> Result<Value, EvalError> {
        serve_pool("mkondo_tumikia", program, args, GRACE, |w: Worker| {
            Box::pin(async move {
                loop {
                    let (stream, _) = match w.listener.accept().await {
                        Ok(Some(c)) => c,
                        Ok(None) => return,
                        // Out of file descriptors and the like: pause instead of spinning.
                        Err(_) => {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                            continue;
                        }
                    };
                    let (tls, starter) = (w.tls.clone(), w.starter);
                    tokio::task::spawn_local(async move {
                        let stream = match tls {
                            Some(cfg) => {
                                match tokio::time::timeout(
                                    CONNECTION_TIMEOUT,
                                    tls::accept(cfg, stream),
                                )
                                .await
                                {
                                    Ok(Ok(s)) => s,
                                    // A failed handshake drops that one connection.
                                    _ => return,
                                }
                            }
                            None => stream,
                        };
                        let mut h = MkondoHandle::new(stream);
                        h.timeout = Some(CONNECTION_TIMEOUT);
                        // An error in one connection's `kazi` ends that connection only.
                        starter.start(vec![Value::Mkondo(Rc::new(RefCell::new(h)))]);
                    });
                }
            })
        })
    }

    /// What a server thread serves with: its listener, TLS, and the program's `kazi_jina`.
    pub(crate) struct Worker {
        pub(crate) listener: LocalListener,
        pub(crate) tls: Option<Arc<rustls::ServerConfig>>,
        pub(crate) starter: crate::kazi_sawia::Starter,
        /// 0 for the first worker of a pool (the one that also serves HTTP/3, once).
        pub(crate) index: usize,
    }

    /// The worker pool of `mkondo_tumikia` and `mkondo_tumikia_http`: check `(sikilizaji,
    /// kazi_jina, idadi_ya_nyuzi, tls?)`, then on that many threads (each with its own engine for
    /// `program`) run `serve` until it ends — when the listener is stopped — give that thread's
    /// tasks `grace` to finish, cancel the rest, and return.
    pub(crate) fn serve_pool<S>(
        name: &str,
        program: &crate::spawn::Shared,
        args: &[Value],
        grace: Duration,
        serve: S,
    ) -> Result<Value, EvalError>
    where
        S: Fn(Worker) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>
            + Clone
            + Send
            + 'static,
    {
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
            .map(|index| {
                let (listener, program, kazi_name) =
                    (Arc::clone(&listener), program.clone(), kazi_name.clone());
                let (tls, serve) = (tls_config.clone(), serve.clone());
                std::thread::spawn(move || {
                    program.with_host(|host| {
                        let Ok(starter) = host.task_starter(&kazi_name) else {
                            return;
                        };
                        let _ = block_on(async move {
                            if let Ok(listener) = listener.local() {
                                serve(Worker {
                                    listener,
                                    tls,
                                    starter,
                                    index,
                                })
                                .await;
                            }
                        });
                        crate::platform::flush_stdout();
                        // The host outlives its tasks: they finish (or are cancelled) here.
                        crate::kazi_sawia::drain_within(grace);
                    })
                })
            })
            .collect();
        for h in handles {
            let _ = h.join();
        }
        Ok(Value::sawa(Value::Tupu))
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
