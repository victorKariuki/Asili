//! `mkondo_tumikia_http`: the HTTP server — HTTP/1.1 and HTTP/2 (`asili_mtandao::http::seva`,
//! on hyper) on a pool of worker threads. Each request runs `kazi_jina(ombi: OmbiHttp) ->
//! JibuHttp` as a `sawia` task, so one thread answers many requests at once: a handler waiting
//! on the network, a channel or `lala` lets the others run. A `JibuHttp` with `mwili_njia`
//! streams its body from a channel (server-sent events: `tukio_sse`).

use std::rc::Rc;
use std::time::Duration;

use asili_mtandao::http::seva::{Handler, Jibu, ServerOptions};

use std::cell::RefCell;

use asili_mtandao::http::seva::WsHandler;

use super::http_thamani::{field, request_to_value, value_to_response};
use super::mkondo::{serve_pool, Worker};
use crate::kazi_sawia::Starter;
use crate::value::{self, EvalError, Value, WsHandle};

/// `mkondo_tumikia_http(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls?, chaguo?: ChaguoSeva) ->
/// Tokeo<Tupu, Neno>`: serve until the listener is stopped (`.simama()`), then let open
/// connections finish (`muda_kuzima`).
pub(crate) fn mkondo_tumikia_http(
    program: &crate::spawn::Shared,
    args: &[Value],
) -> Result<Value, EvalError> {
    let o = args.get(4).unwrap_or(&Value::Hamna);
    let seconds = |v: &Value| {
        value::as_f64(v)
            .filter(|s| s.is_finite() && *s >= 0.0)
            .map(Duration::from_secs_f64)
    };
    let mut opts = ServerOptions::default();
    if let Some(n) = value::as_f64(field(o, "kikomo_mwili")) {
        opts.body_limit = n.max(0.0) as usize;
    }
    if let Some(d) = seconds(field(o, "muda")) {
        opts.header_timeout = d;
    }
    if let Value::Ukweli(b) = field(o, "bana") {
        opts.compress = *b;
    }
    let http3 = matches!(field(o, "h3"), Value::Ukweli(true));
    if let Some(d) = seconds(field(o, "muda_kuzima")) {
        opts.grace = d;
    }
    let grace = opts.grace;
    serve_pool(
        "mkondo_tumikia_http",
        program,
        args,
        grace,
        move |w: Worker| {
            let mut opts = opts.clone();
            opts.tls = w.tls;
            // HTTP/3 is served once per address: by the pool's first worker.
            opts.http3 = http3 && w.index == 0;
            let starter = w.starter;
            let handler: Handler = Rc::new(move |ombi| {
                Box::pin(async move {
                    let task = starter.start(vec![request_to_value(ombi)]);
                    task.finished().await;
                    match task.result() {
                        Some(Ok(v)) => match value_to_response(&v) {
                            Some(mut jibu) => {
                                if let Value::Neno(name) = field(&v, "ws") {
                                    match starter.named(name) {
                                        Ok(ws) => jibu.websocket = Some(ws_handler(ws)),
                                        Err(_) => {
                                            jibu = Jibu::text(500, "kazi ya WebSocket haijulikani")
                                        }
                                    }
                                }
                                jibu
                            }
                            None => Jibu::text(500, "jibu batili kutoka kwa kazi_jina"),
                        },
                        _ => Jibu::text(500, "hitilafu ya ndani"),
                    }
                })
            });
            Box::pin(asili_mtandao::http::seva::serve(w.listener, opts, handler))
        },
    )
}

/// The handler of a WebSocket: runs the program's function `starter` with the connection, as a
/// task, and returns once that task has.
fn ws_handler(starter: Starter) -> WsHandler {
    Box::new(move |conn| {
        Box::pin(async move {
            let ws = Value::MkondoWs(Rc::new(RefCell::new(WsHandle(Some(conn)))));
            starter.start(vec![ws]).finished().await;
        })
    })
}
