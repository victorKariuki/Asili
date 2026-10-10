//! The HTTP client on hyper: one request/response exchange over a pooled connection — HTTP/1.1
//! or HTTP/2 (by ALPN), TLS with the Mozilla roots or a private CA and optional client
//! certificate, HTTP (absolute-form or CONNECT) and SOCKS5 proxies (also from the environment),
//! request bodies from memory or a file, and gzip/brotli decoding with a size limit. Redirects,
//! retries and cookies are the caller's (they are policy, and differ per program option).
//!
//! Connections are pooled per thread (they belong to that thread's event loop) and reused while
//! the server keeps them open; a request that fails on a reused connection is sent once more on a
//! fresh one.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::{Frame, Incoming};
use hyper::header::{HeaderName, HeaderValue};
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

use super::Local;
use crate::stream::{connect_tcp, ConnectOptions};
use crate::{tls, Stream};

/// How to reach servers: the same settings share connections.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Transport {
    /// A proxy URL (`http://h:p`, `socks5://h:p`, with `user:pass@` allowed); `""`: none even if
    /// the environment names one; `None`: the environment's (`HTTPS_PROXY`, `HTTP_PROXY`,
    /// `ALL_PROXY`, `NO_PROXY`).
    pub proxy: Option<String>,
    /// 4 or 6: that IP family only.
    pub ip_family: Option<u8>,
    /// Trust only the certificates in this PEM file (default: the Mozilla roots).
    pub ca_file: Option<String>,
    /// A client certificate and its key (PEM files).
    pub client_cert: Option<(String, String)>,
    /// Give up connecting (lookup, connect, TLS) after this long, in milliseconds.
    pub connect_timeout_ms: Option<u64>,
}

/// A request body; rebuilt for every attempt.
#[derive(Clone, Debug)]
pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    /// Sent from the file without reading it all into memory.
    File(String),
}

enum ReqBody {
    Empty,
    Bytes(Option<Bytes>),
    File(std::fs::File, u64),
}

impl hyper::body::Body for ReqBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        use std::io::Read;
        match self.get_mut() {
            ReqBody::Empty => Poll::Ready(None),
            ReqBody::Bytes(b) => Poll::Ready(b.take().map(|b| Ok(Frame::data(b)))),
            ReqBody::File(f, left) => {
                if *left == 0 {
                    return Poll::Ready(None);
                }
                let mut buf = vec![0u8; (*left).min(64 * 1024) as usize];
                match f.read(&mut buf) {
                    Ok(0) => Poll::Ready(Some(Err(io::ErrorKind::UnexpectedEof.into()))),
                    Ok(n) => {
                        buf.truncate(n);
                        *left -= n as u64;
                        Poll::Ready(Some(Ok(Frame::data(Bytes::from(buf)))))
                    }
                    Err(e) => Poll::Ready(Some(Err(e))),
                }
            }
        }
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        match self {
            ReqBody::Empty => hyper::body::SizeHint::with_exact(0),
            ReqBody::Bytes(b) => {
                hyper::body::SizeHint::with_exact(b.as_ref().map_or(0, |b| b.len() as u64))
            }
            ReqBody::File(_, left) => hyper::body::SizeHint::with_exact(*left),
        }
    }
}

fn open(body: &Body) -> Result<(ReqBody, Option<u64>), String> {
    Ok(match body {
        Body::Empty => (ReqBody::Empty, None),
        Body::Bytes(b) => (
            ReqBody::Bytes(Some(Bytes::from(b.clone()))),
            Some(b.len() as u64),
        ),
        Body::File(path) => {
            let f = std::fs::File::open(path).map_err(|e| format!("faili {path}: {e}"))?;
            let n = f
                .metadata()
                .map_err(|e| format!("faili {path}: {e}"))?
                .len();
            (ReqBody::File(f, n), Some(n))
        }
    })
}

/// A `multipart/form-data` body: its `Content-Type` and bytes. Files are read here.
pub fn multipart(
    text: &[(String, String)],
    files: &[(String, String)],
) -> Result<(String, Vec<u8>), String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let boundary = format!("asili-{:x}{:x}", std::process::id(), nanos);
    let quote = |s: &str| s.replace('"', "%22").replace(['\r', '\n'], "");
    let mut out = Vec::new();
    for (k, v) in text {
        out.extend_from_slice(
            format!(
                "--{boundary}\r\ncontent-disposition: form-data; name=\"{}\"\r\n\r\n{v}\r\n",
                quote(k)
            )
            .as_bytes(),
        );
    }
    for (k, path) in files {
        let data = std::fs::read(path).map_err(|e| format!("fomu_faili {path}: {e}"))?;
        let name = std::path::Path::new(path)
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        out.extend_from_slice(
            format!(
                "--{boundary}\r\ncontent-disposition: form-data; name=\"{}\"; filename=\"{}\"\r\ncontent-type: application/octet-stream\r\n\r\n",
                quote(k),
                quote(&name)
            )
            .as_bytes(),
        );
        out.extend_from_slice(&data);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Ok((format!("multipart/form-data; boundary={boundary}"), out))
}

/// A request to send once.
pub struct Request<'a> {
    pub method: &'a str,
    pub url: &'a Url,
    pub headers: &'a [(String, String)],
    pub body: &'a Body,
}

/// The server's answer, its body not yet read.
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub version: &'static str,
    /// Every header, lowercase names, in order.
    pub headers: Vec<(String, String)>,
    body: Incoming,
    encoding: Option<String>,
}

enum Sender {
    H1(hyper::client::conn::http1::SendRequest<ReqBody>),
    H2(hyper::client::conn::http2::SendRequest<ReqBody>),
}

impl Sender {
    async fn ready(&mut self) -> bool {
        match self {
            Sender::H1(s) => s.ready().await.is_ok(),
            Sender::H2(s) => s.ready().await.is_ok(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct PoolKey {
    https: bool,
    host: String,
    port: u16,
    transport: Transport,
}

/// Idle connections kept per server and thread.
const IDLE_PER_HOST: usize = 10;

thread_local! {
    static POOL: RefCell<HashMap<PoolKey, Vec<Sender>>> = RefCell::new(HashMap::new());
}

/// Send `req` and return the answer's head. Errors are messages for the program.
pub async fn send(t: &Transport, req: &Request<'_>) -> Result<Response, String> {
    let url = req.url;
    let host = url.host_str().ok_or("anwani haina mwenyeji")?.to_string();
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let https = url.scheme() == "https";
    let port = url.port_or_known_default().ok_or("anwani haina mlango")?;
    let key = PoolKey {
        https,
        host: host.clone(),
        port,
        transport: t.clone(),
    };
    // A reused connection may have been closed by the server meanwhile: once more on a new one.
    loop {
        let pooled = POOL.with(|p| p.borrow_mut().get_mut(&key).and_then(Vec::pop));
        let (mut sender, reused, via_proxy) = match pooled {
            Some(mut s) => {
                if !s.ready().await {
                    continue;
                }
                (s, true, false)
            }
            None => {
                let (s, via) = connect(t, url, &host, port, https).await?;
                (s, false, via)
            }
        };
        match exchange(&mut sender, req, via_proxy).await {
            Ok(resp) => {
                POOL.with(|p| {
                    let mut p = p.borrow_mut();
                    let idle = p.entry(key).or_default();
                    if idle.len() < IDLE_PER_HOST {
                        idle.push(sender);
                    }
                });
                return Ok(resp);
            }
            Err(_) if reused => continue,
            Err(e) => return Err(e),
        }
    }
}

async fn exchange(
    sender: &mut Sender,
    req: &Request<'_>,
    via_http_proxy: bool,
) -> Result<Response, String> {
    let url = req.url;
    let method = hyper::Method::from_bytes(req.method.trim().to_ascii_uppercase().as_bytes())
        .map_err(|_| "njia si sahihi".to_string())?;
    let h2 = matches!(sender, Sender::H2(_));
    let target = if h2 || via_http_proxy {
        url.as_str().to_string()
    } else {
        let mut t = url.path().to_string();
        if let Some(q) = url.query() {
            t.push('?');
            t.push_str(q);
        }
        t
    };
    let mut b = hyper::Request::builder().method(method).uri(target);
    let mut have: Vec<String> = Vec::new();
    for (k, v) in req.headers {
        let n = HeaderName::from_bytes(k.trim().as_bytes())
            .map_err(|_| format!("jina la kichwa si sahihi: {k}"))?;
        let v =
            HeaderValue::from_str(v).map_err(|_| format!("thamani ya kichwa '{k}' si sahihi"))?;
        have.push(n.as_str().to_string());
        b = b.header(n, v);
    }
    let absent = |n: &str| !have.iter().any(|h| h == n);
    if !h2 && absent("host") {
        let mut authority = url.host_str().unwrap_or("").to_string();
        if let Some(p) = url.port() {
            authority.push_str(&format!(":{p}"));
        }
        b = b.header("host", authority);
    }
    if absent("user-agent") {
        b = b.header("user-agent", concat!("asili/", env!("CARGO_PKG_VERSION")));
    }
    if absent("accept") {
        b = b.header("accept", "*/*");
    }
    if absent("accept-encoding") {
        b = b.header("accept-encoding", "gzip, br");
    }
    let (body, len) = open(req.body)?;
    if let Some(n) = len {
        if absent("content-length") && !h2 && (n > 0 || !matches!(req.method, "GET" | "HEAD")) {
            b = b.header("content-length", n);
        }
    }
    let request = b.body(body).map_err(|e| e.to_string())?;
    let resp = match sender {
        Sender::H1(s) => s.send_request(request).await,
        Sender::H2(s) => s.send_request(request).await,
    }
    .map_err(|e| format!("ombi: {e}"))?;
    let (parts, body) = resp.into_parts();
    let reason = parts
        .extensions
        .get::<hyper::ext::ReasonPhrase>()
        .map(|r| String::from_utf8_lossy(r.as_bytes()).into_owned())
        .or_else(|| parts.status.canonical_reason().map(str::to_string))
        .unwrap_or_default();
    let encoding = parts
        .headers
        .get(hyper::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_ascii_lowercase());
    Ok(Response {
        status: parts.status.as_u16(),
        reason,
        version: super::version_name(parts.version),
        headers: super::header_pairs(&parts.headers),
        body,
        encoding,
    })
}

/// A connection to `url`'s server, and whether requests on it go by absolute-form (never so
/// far: proxies tunnel).
async fn connect(
    t: &Transport,
    url: &Url,
    host: &str,
    port: u16,
    https: bool,
) -> Result<(Sender, bool), String> {
    let limit = t.connect_timeout_ms.map(Duration::from_millis);
    let work = async {
        let proxy = proxy_for(t, url);
        let via_http_proxy = false;
        let tcp = match &proxy {
            Some(p) => through_proxy(p, host, port, t.ip_family).await?,
            None => connect_tcp(host, port, t.ip_family).await?,
        };
        let stream = if https {
            let opts = ConnectOptions {
                tls: true,
                ca_file: t.ca_file.clone(),
                client_cert: t.client_cert.clone(),
                alpn: vec![b"h2".to_vec(), b"http/1.1".to_vec()],
                ..Default::default()
            };
            tls::connect(tcp, host, &opts).await?
        } else {
            Stream::Tcp(tcp)
        };
        let h2 = stream.alpn().as_deref() == Some(b"h2");
        let io = TokioIo::new(stream);
        let sender = if h2 {
            let (s, conn) = hyper::client::conn::http2::handshake(Local, io)
                .await
                .map_err(|e| format!("HTTP/2: {e}"))?;
            tokio::task::spawn_local(async move {
                let _ = conn.await;
            });
            Sender::H2(s)
        } else {
            let (s, conn) = hyper::client::conn::http1::handshake(io)
                .await
                .map_err(|e| format!("HTTP: {e}"))?;
            tokio::task::spawn_local(async move {
                let _ = conn.await;
            });
            Sender::H1(s)
        };
        Ok::<_, String>((sender, via_http_proxy))
    };
    match limit {
        Some(d) => tokio::time::timeout(d, work)
            .await
            .map_err(|_| "muda umekwisha".to_string())?,
        None => work.await,
    }
}

/// The proxy to use for `url`, if any.
fn proxy_for(t: &Transport, url: &Url) -> Option<String> {
    match &t.proxy {
        Some(p) if p.is_empty() => None,
        Some(p) => Some(p.clone()),
        None => {
            let host = url.host_str()?;
            if no_proxy(host) {
                return None;
            }
            let get = |names: &[&str]| {
                names
                    .iter()
                    .find_map(|n| std::env::var(n).ok().filter(|v| !v.is_empty()))
            };
            if url.scheme() == "https" {
                get(&["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"])
            } else {
                get(&["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"])
            }
        }
    }
}

/// Whether `NO_PROXY` exempts `host` (`*`, names, `.suffix`, IPv4 `a.b.c.d/n`).
fn no_proxy(host: &str) -> bool {
    let list = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .unwrap_or_default();
    let ip: Option<std::net::Ipv4Addr> = host.parse().ok();
    list.split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .any(|e| {
            if e == "*" {
                return true;
            }
            if let (Some((net, bits)), Some(ip)) = (e.split_once('/'), ip) {
                if let (Ok(net), Ok(bits)) =
                    (net.parse::<std::net::Ipv4Addr>(), bits.parse::<u32>())
                {
                    let mask = if bits == 0 {
                        0
                    } else {
                        u32::MAX << (32 - bits.min(32))
                    };
                    return u32::from(ip) & mask == u32::from(net) & mask;
                }
            }
            let e = e.trim_start_matches('.');
            host == e || host.ends_with(&format!(".{e}"))
        })
}

/// A TCP stream to `host:port` through `proxy`: an HTTP proxy is asked to CONNECT (also for
/// plain HTTP, so the proxy only relays bytes), a SOCKS5 proxy to connect.
async fn through_proxy(
    proxy: &str,
    host: &str,
    port: u16,
    family: Option<u8>,
) -> Result<TcpStream, String> {
    let p = Url::parse(proxy).map_err(|e| format!("wakala {proxy}: {e}"))?;
    let phost = p
        .host_str()
        .ok_or_else(|| format!("wakala {proxy}: hakuna mwenyeji"))?;
    let pphost = phost.trim_start_matches('[').trim_end_matches(']');
    let pport = p.port_or_known_default().unwrap_or(match p.scheme() {
        "socks5" | "socks5h" => 1080,
        "https" => 443,
        _ => 80,
    });
    let decode = |s: &str| {
        percent_encoding::percent_decode_str(s)
            .decode_utf8_lossy()
            .into_owned()
    };
    let (user, pass) = (decode(p.username()), p.password().map(decode));
    let mut tcp = connect_tcp(pphost, pport, family).await?;
    match p.scheme() {
        "socks5" | "socks5h" => {
            let target = (host, port);
            let s = if user.is_empty() {
                tokio_socks::tcp::Socks5Stream::connect_with_socket(tcp, target).await
            } else {
                tokio_socks::tcp::Socks5Stream::connect_with_password_and_socket(
                    tcp,
                    target,
                    &user,
                    pass.as_deref().unwrap_or(""),
                )
                .await
            };
            Ok(s.map_err(|e| format!("wakala {proxy}: {e}"))?.into_inner())
        }
        "http" | "https" => {
            let mut req = format!("CONNECT {host}:{port} HTTP/1.1\r\nhost: {host}:{port}\r\n");
            if !user.is_empty() {
                use base64::Engine;
                let cred = base64::engine::general_purpose::STANDARD
                    .encode(format!("{user}:{}", pass.as_deref().unwrap_or("")));
                req.push_str(&format!("proxy-authorization: Basic {cred}\r\n"));
            }
            req.push_str("\r\n");
            tcp.write_all(req.as_bytes())
                .await
                .map_err(|e| format!("wakala {proxy}: {e}"))?;
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if head.len() > 16 * 1024
                    || tcp.read(&mut byte).await.map_err(|e| e.to_string())? == 0
                {
                    return Err(format!("wakala {proxy}: jibu la CONNECT halikukamilika"));
                }
                head.push(byte[0]);
            }
            let line = String::from_utf8_lossy(&head);
            let status = line.split_whitespace().nth(1).unwrap_or("");
            if !status.starts_with('2') {
                return Err(format!(
                    "wakala {proxy}: CONNECT imekataliwa ({})",
                    line.lines().next().unwrap_or("")
                ));
            }
            Ok(tcp)
        }
        other => Err(format!("wakala: mpango '{other}' haujulikani")),
    }
}

/// The size limit was passed while reading a body.
#[derive(Debug)]
struct LimitExceeded;

impl std::fmt::Display for LimitExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("kikomo")
    }
}

impl std::error::Error for LimitExceeded {}

/// Decoded bytes on their way to the caller's sink, counted against the limit.
struct Counted<'a> {
    sink: &'a mut dyn FnMut(&[u8]) -> io::Result<()>,
    seen: u64,
    limit: u64,
}

impl Write for Counted<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.seen += buf.len() as u64;
        if self.seen > self.limit {
            return Err(io::Error::other(LimitExceeded));
        }
        (self.sink)(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

enum Decoder<'a> {
    Plain(Counted<'a>),
    Gzip(Box<flate2::write::GzDecoder<Counted<'a>>>),
    Brotli(Box<brotli::DecompressorWriter<Counted<'a>>>),
}

impl Response {
    /// Read the body to its end, decoded (gzip, brotli), handing each piece to `sink`; at most
    /// `limit` decoded bytes.
    pub async fn read_body(
        mut self,
        limit: u64,
        sink: &mut dyn FnMut(&[u8]) -> io::Result<()>,
    ) -> Result<(), String> {
        let counted = Counted {
            sink,
            seen: 0,
            limit,
        };
        let mut dec = match self.encoding.as_deref() {
            Some("gzip" | "x-gzip") => {
                Decoder::Gzip(Box::new(flate2::write::GzDecoder::new(counted)))
            }
            Some("br") => Decoder::Brotli(Box::new(brotli::DecompressorWriter::new(counted, 4096))),
            _ => Decoder::Plain(counted),
        };
        let fail = |e: io::Error| {
            if e.get_ref().is_some_and(|i| i.is::<LimitExceeded>()) {
                format!("jibu limezidi kikomo cha baiti {limit}")
            } else {
                format!("kusoma jibu: {e}")
            }
        };
        let mut any = false;
        while let Some(frame) = self.body.frame().await {
            let frame = frame.map_err(|e| format!("kusoma jibu: {e}"))?;
            let Ok(data) = frame.into_data() else {
                continue;
            };
            any = true;
            match &mut dec {
                Decoder::Plain(w) => w.write_all(&data),
                Decoder::Gzip(w) => w.write_all(&data),
                Decoder::Brotli(w) => w.write_all(&data),
            }
            .map_err(fail)?;
        }
        if any {
            match dec {
                Decoder::Plain(_) => Ok(()),
                Decoder::Gzip(w) => w.finish().map(|_| ()),
                Decoder::Brotli(mut w) => w.flush().and_then(|()| w.close()),
            }
            .map_err(fail)?;
        }
        Ok(())
    }
}
