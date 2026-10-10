//! The HTTP server: HTTP/1.1 and HTTP/2 (by ALPN over TLS, or by the client's preface in the
//! clear) on hyper, one connection per local task. Each request's body is read whole (up to a
//! limit) and handed to the handler; the answer is a whole body (compressed when the client
//! accepts it and it is worth it) or a stream of chunks. Stopping the listener stops accepting,
//! lets open connections finish their requests (up to a grace period), then ends them.

use std::cell::Cell;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Limited};
use hyper::body::{Frame, Incoming};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use tokio::sync::Notify;
use tokio::task::JoinSet;

use super::bana;
use crate::{tls, LocalListener, Stream};

/// A request, as the handler receives it.
#[derive(Debug, Clone)]
pub struct Ombi {
    /// `GET`, `POST`, ...
    pub method: String,
    /// The path and query as sent (`/bidhaa?id=7`).
    pub target: String,
    /// The path, percent-decoded (`/bidhaa`).
    pub path: String,
    /// The query's parameters, decoded, in order.
    pub query: Vec<(String, String)>,
    /// `HTTP/1.1`, `HTTP/2`, ...
    pub version: &'static str,
    /// Header names (lowercase) and values, in order.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// The client's address.
    pub peer: String,
}

/// Chunks of a streamed answer; the answer ends when the stream does.
pub type Chunks = Pin<Box<dyn futures_core::Stream<Item = Vec<u8>>>>;

/// An answer's body.
pub enum JibuMwili {
    Full(Vec<u8>),
    Stream(Chunks),
}

/// An answer, as the handler gives it.
pub struct Jibu {
    pub status: u16,
    /// A reason phrase other than the standard one.
    pub reason: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: JibuMwili,
}

impl Jibu {
    /// A plain-text answer.
    pub fn text(status: u16, body: &str) -> Jibu {
        Jibu {
            status,
            reason: None,
            headers: vec![(
                "content-type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            body: JibuMwili::Full(body.as_bytes().to_vec()),
        }
    }
}

/// Answers requests (on the server's thread).
pub type Handler = Rc<dyn Fn(Ombi) -> Pin<Box<dyn Future<Output = Jibu>>>>;

/// How to serve.
#[derive(Clone, Debug)]
pub struct ServerOptions {
    /// Largest request body accepted (larger: `413`).
    pub body_limit: usize,
    /// Time to complete TLS and send a request's headers.
    pub header_timeout: Duration,
    /// Compress answers the client accepts compressed.
    pub compress: bool,
    /// After the listener stops, how long open connections may take to finish.
    pub grace: Duration,
    /// Serve TLS with this configuration (offering HTTP/2 by ALPN).
    pub tls: Option<Arc<rustls::ServerConfig>>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        ServerOptions {
            body_limit: 16 << 20,
            header_timeout: Duration::from_secs(30),
            compress: true,
            grace: Duration::from_secs(10),
            tls: None,
        }
    }
}

/// Bodies smaller than this are not worth compressing.
const MIN_COMPRESS: usize = 860;

use super::Local;

/// Set once the server stops; connections then finish gracefully.
#[derive(Default)]
struct Stop {
    stopped: Cell<bool>,
    notify: Notify,
}

impl Stop {
    fn stop(&self) {
        self.stopped.set(true);
        self.notify.notify_waiters();
    }

    async fn wait(&self) {
        loop {
            let n = self.notify.notified();
            if self.stopped.get() {
                return;
            }
            n.await;
        }
    }
}

/// Serve `listener`'s connections with `handler` until the listener is stopped and its open
/// connections have finished (or the grace period ran out). Call inside a `LocalSet`.
pub async fn serve(listener: LocalListener, mut opts: ServerOptions, handler: Handler) {
    if let Some(cfg) = opts.tls.take() {
        // Offer HTTP/2 and HTTP/1.1, in that order, unless the configuration says otherwise.
        let mut cfg = (*cfg).clone();
        if cfg.alpn_protocols.is_empty() {
            cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        }
        opts.tls = Some(Arc::new(cfg));
    }
    let opts = Rc::new(opts);
    let stop = Rc::new(Stop::default());
    let mut conns = JoinSet::new();
    loop {
        while conns.try_join_next().is_some() {}
        let (stream, peer) = match listener.accept().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            // Out of file descriptors and the like: pause instead of spinning.
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }
        };
        conns.spawn_local(connection(
            stream,
            peer,
            opts.clone(),
            handler.clone(),
            stop.clone(),
        ));
    }
    stop.stop();
    let grace = opts.grace;
    let _ = tokio::time::timeout(grace, async { while conns.join_next().await.is_some() {} }).await;
    conns.abort_all();
    while conns.join_next().await.is_some() {}
}

async fn connection(
    stream: Stream,
    peer: String,
    opts: Rc<ServerOptions>,
    handler: Handler,
    stop: Rc<Stop>,
) {
    let stream = match &opts.tls {
        Some(cfg) => {
            match tokio::time::timeout(opts.header_timeout, tls::accept(cfg.clone(), stream)).await
            {
                Ok(Ok(s)) => s,
                _ => return,
            }
        }
        None => stream,
    };
    let io = TokioIo::new(stream);
    let svc_opts = opts.clone();
    let service = hyper::service::service_fn(move |req: Request<Incoming>| {
        let (peer, opts, handler) = (peer.clone(), svc_opts.clone(), handler.clone());
        async move { Ok::<_, Infallible>(respond(req, peer, &opts, &handler).await) }
    });
    let mut builder = auto::Builder::new(Local);
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(opts.header_timeout)
        .keep_alive(true);
    builder
        .http2()
        .timer(TokioTimer::new())
        .max_concurrent_streams(256)
        .keep_alive_interval(Some(Duration::from_secs(30)));
    let conn = builder.serve_connection_with_upgrades(io, service);
    tokio::pin!(conn);
    tokio::select! {
        _ = conn.as_mut() => {}
        _ = stop.wait() => {
            conn.as_mut().graceful_shutdown();
            let _ = conn.await;
        }
    }
}

/// A response body: whole, or a stream of chunks (on the server's thread; not `Send`).
pub(crate) enum Body {
    Full(Option<Bytes>),
    Stream(Chunks),
}

impl hyper::body::Body for Body {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        match self.get_mut() {
            Body::Full(b) => Poll::Ready(b.take().map(|b| Ok(Frame::data(b)))),
            Body::Stream(s) => s
                .as_mut()
                .poll_next(cx)
                .map(|c| c.map(|c| Ok(Frame::data(Bytes::from(c))))),
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(self, Body::Full(None))
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        match self {
            Body::Full(b) => {
                hyper::body::SizeHint::with_exact(b.as_ref().map_or(0, |b| b.len() as u64))
            }
            Body::Stream(_) => hyper::body::SizeHint::default(),
        }
    }
}

fn full(data: Vec<u8>) -> Body {
    Body::Full(Some(Bytes::from(data)))
}

fn plain(status: StatusCode, msg: &str) -> Response<Body> {
    let mut r = Response::new(full(msg.as_bytes().to_vec()));
    *r.status_mut() = status;
    r.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    r
}

async fn respond(
    req: Request<Incoming>,
    peer: String,
    opts: &ServerOptions,
    handler: &Handler,
) -> Response<Body> {
    let (parts, body) = req.into_parts();
    let body = match Limited::new(body, opts.body_limit).collect().await {
        Ok(c) => c.to_bytes().to_vec(),
        Err(e)
            if e.downcast_ref::<http_body_util::LengthLimitError>()
                .is_some() =>
        {
            return plain(StatusCode::PAYLOAD_TOO_LARGE, "mwili wa ombi ni mkubwa mno")
        }
        Err(_) => return plain(StatusCode::BAD_REQUEST, "mwili wa ombi haukukamilika"),
    };
    let accept_encoding = parts
        .headers
        .get(hyper::header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let ombi = Ombi {
        method: parts.method.as_str().to_string(),
        target: parts
            .uri
            .path_and_query()
            .map_or("/", |p| p.as_str())
            .to_string(),
        path: percent_encoding::percent_decode_str(parts.uri.path())
            .decode_utf8_lossy()
            .into_owned(),
        query: parts
            .uri
            .query()
            .map(|q| form_urlencoded::parse(q.as_bytes()).into_owned().collect())
            .unwrap_or_default(),
        version: super::version_name(parts.version),
        headers: super::header_pairs(&parts.headers),
        body,
        peer,
    };
    let jibu = handler(ombi).await;
    answer(jibu, &accept_encoding, opts.compress)
}

/// The response for `jibu`: invalid header names or values are left out.
fn answer(jibu: Jibu, accept_encoding: &str, compress: bool) -> Response<Body> {
    let mut headers = hyper::HeaderMap::new();
    for (k, v) in &jibu.headers {
        if let (Ok(k), Ok(v)) = (
            HeaderName::try_from(k.as_str()),
            HeaderValue::try_from(v.as_str()),
        ) {
            headers.append(k, v);
        }
    }
    let status = StatusCode::from_u16(jibu.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = match jibu.body {
        JibuMwili::Full(data) => {
            let encoding = (compress
                && data.len() >= MIN_COMPRESS
                && !headers.contains_key(hyper::header::CONTENT_ENCODING)
                && bana::compressible(
                    headers
                        .get(hyper::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok()),
                ))
            .then(|| bana::choose(accept_encoding))
            .flatten();
            match encoding.and_then(|e| Some((e, bana::compress(e, &data)?))) {
                Some((e, packed)) => {
                    headers.insert(hyper::header::CONTENT_ENCODING, HeaderValue::from_static(e));
                    headers.append(
                        hyper::header::VARY,
                        HeaderValue::from_static("accept-encoding"),
                    );
                    headers.remove(hyper::header::CONTENT_LENGTH);
                    full(packed)
                }
                None => full(data),
            }
        }
        JibuMwili::Stream(chunks) => Body::Stream(chunks),
    };
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    if let Some(reason) = jibu.reason {
        if let Ok(r) = hyper::ext::ReasonPhrase::try_from(reason.into_bytes()) {
            response.extensions_mut().insert(r);
        }
    }
    response
}
