//! A connected stream (TCP, TLS over TCP, or a Unix socket) and connecting one.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio_rustls::TlsStream;

/// A connected byte stream.
pub enum Stream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
    #[cfg(unix)]
    Unix(UnixStream),
}

macro_rules! each {
    ($self:expr, $s:ident => $e:expr) => {
        match $self {
            Stream::Tcp($s) => $e,
            Stream::Tls($s) => $e,
            #[cfg(unix)]
            Stream::Unix($s) => $e,
        }
    };
}

impl AsyncRead for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        each!(self.get_mut(), s => Pin::new(s).poll_read(cx, buf))
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        each!(self.get_mut(), s => Pin::new(s).poll_write(cx, buf))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        each!(self.get_mut(), s => Pin::new(s).poll_flush(cx))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        each!(self.get_mut(), s => Pin::new(s).poll_shutdown(cx))
    }
}

impl Stream {
    fn tcp(&self) -> Option<&TcpStream> {
        match self {
            Stream::Tcp(s) => Some(s),
            Stream::Tls(s) => Some(s.get_ref().0),
            #[cfg(unix)]
            Stream::Unix(_) => None,
        }
    }

    /// Whether the stream is TLS.
    pub fn is_tls(&self) -> bool {
        matches!(self, Stream::Tls(_))
    }

    /// The protocol TLS negotiated (ALPN: `h2`, `http/1.1`), if any.
    pub fn alpn(&self) -> Option<Vec<u8>> {
        match self {
            Stream::Tls(s) => s.get_ref().1.alpn_protocol().map(<[u8]>::to_vec),
            _ => None,
        }
    }

    /// The other end's address.
    pub fn peer_addr(&self) -> String {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => unix_addr(s.peer_addr().ok()),
            _ => self
                .tcp()
                .and_then(|t| t.peer_addr().ok())
                .map_or_else(String::new, |a| a.to_string()),
        }
    }

    /// This end's address.
    pub fn local_addr(&self) -> String {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => unix_addr(s.local_addr().ok()),
            _ => self
                .tcp()
                .and_then(|t| t.local_addr().ok())
                .map_or_else(String::new, |a| a.to_string()),
        }
    }

    /// End TLS politely without waiting (for a drop): queue `close_notify` and write it if the
    /// socket takes it now. A bare TCP close looks like a truncation attack to the peer.
    pub fn close_now(&mut self) {
        if let Stream::Tls(s) = self {
            match &mut **s {
                TlsStream::Client(c) => c.get_mut().1.send_close_notify(),
                TlsStream::Server(c) => c.get_mut().1.send_close_notify(),
            }
            let mut cx = Context::from_waker(std::task::Waker::noop());
            let _ = Pin::new(&mut **s).poll_flush(&mut cx);
        }
    }
}

#[cfg(unix)]
pub(crate) fn unix_addr(a: Option<tokio::net::unix::SocketAddr>) -> String {
    match a.as_ref().and_then(|a| a.as_pathname()) {
        Some(p) => format!("unix:{}", p.display()),
        None => "unix:".to_string(),
    }
}

/// How to connect.
#[derive(Clone, Debug, Default)]
pub struct ConnectOptions {
    /// Speak TLS, checking the server's certificate.
    pub tls: bool,
    /// The name the certificate must carry (default: the host).
    pub server_name: Option<String>,
    /// Give up after this long (lookup, connect and handshake together).
    pub timeout: Option<Duration>,
    /// Trust only the certificates in this PEM file (default: the Mozilla roots).
    pub ca_file: Option<String>,
    /// A client certificate and its key (PEM files), for mutual TLS.
    pub client_cert: Option<(String, String)>,
    /// 4 or 6: that IP family only.
    pub ip_family: Option<u8>,
    /// TLS protocols to offer (ALPN), most preferred first.
    pub alpn: Vec<Vec<u8>>,
}

/// `"mwenyeji:mlango"` / `"[::1]:mlango"` as `(mwenyeji, mlango)`.
pub fn split_host_port(addr: &str) -> Result<(String, u16), String> {
    let bad = || format!("anwani batili (mwenyeji:mlango): {addr}");
    let (host, port) = match addr.strip_prefix('[') {
        Some(rest) => rest.split_once("]:").ok_or_else(bad)?,
        None => addr.rsplit_once(':').ok_or_else(bad)?,
    };
    let port = port.parse().map_err(|_| bad())?;
    Ok((host.to_string(), port))
}

/// Connect to `addr`.
pub async fn connect(addr: &str, opts: &ConnectOptions) -> Result<Stream, String> {
    let work = async {
        if let Some(path) = addr.strip_prefix("unix:") {
            #[cfg(unix)]
            return UnixStream::connect(path)
                .await
                .map(Stream::Unix)
                .map_err(|e| format!("kuunganisha {addr}: {e}"));
            #[cfg(not(unix))]
            return Err(format!("soketi za unix hazipatikani hapa: {path}"));
        }
        let (host, port) = split_host_port(addr)?;
        let tcp = connect_tcp(&host, port, opts.ip_family).await?;
        if !opts.tls {
            return Ok(Stream::Tcp(tcp));
        }
        let name = opts.server_name.clone().unwrap_or(host);
        crate::tls::connect(tcp, &name, opts).await
    };
    match opts.timeout {
        Some(d) => tokio::time::timeout(d, work)
            .await
            .map_err(|_| format!("kuunganisha {addr}: muda umekwisha"))?,
        None => work.await,
    }
}

/// A TCP connection to the first of `host`'s addresses that answers.
pub(crate) async fn connect_tcp(
    host: &str,
    port: u16,
    family: Option<u8>,
) -> Result<TcpStream, String> {
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("kutafuta {host}: {e}"))?
        .filter(|a| match family {
            Some(4) => a.is_ipv4(),
            Some(6) => a.is_ipv6(),
            _ => true,
        })
        .collect();
    let mut last = format!("kutafuta {host}: hakuna anwani");
    for a in addrs {
        match TcpStream::connect(a).await {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                return Ok(s);
            }
            Err(e) => last = format!("kuunganisha {a}: {e}"),
        }
    }
    Err(last)
}
