//! WebSocket (RFC 6455): a client connects to `ws://` and `wss://` addresses; a server upgrades
//! an HTTP/1.1 request (see `http::seva`). Both ends then exchange messages on one stream;
//! pings are answered by the protocol layer.

use std::time::Duration;

use hyper_util::rt::TokioIo;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;
pub use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::stream::ConnectOptions;

/// Any stream a WebSocket can run over (TCP, TLS, a Unix socket, an upgraded connection).
pub trait Io: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> Io for T {}

/// An open WebSocket.
pub type Ws = WebSocketStream<Box<dyn Io>>;

/// Connect to `url` (`ws://host[:port]/path` or `wss://...`).
pub async fn connect(url: &str, opts: &ConnectOptions) -> Result<Ws, String> {
    let u = url::Url::parse(url).map_err(|e| format!("anwani si sahihi: {e}"))?;
    let tls = match u.scheme() {
        "ws" => false,
        "wss" => true,
        other => {
            return Err(format!(
                "mpango '{other}' si wa WebSocket (ws:// au wss://)"
            ))
        }
    };
    let host = u.host_str().ok_or("anwani haina mwenyeji")?;
    let port = u.port_or_known_default().ok_or("anwani haina mlango")?;
    let addr = format!("{host}:{port}");
    let mut o = opts.clone();
    o.tls = tls;
    let limit = o.timeout;
    let work = async {
        let stream = crate::connect(&addr, &o).await?;
        let io: Box<dyn Io> = Box::new(stream);
        tokio_tungstenite::client_async(url, io)
            .await
            .map(|(ws, _)| ws)
            .map_err(|e| format!("WebSocket: {e}"))
    };
    match limit {
        Some(d) => tokio::time::timeout(d, work)
            .await
            .map_err(|_| "muda umekwisha".to_string())?,
        None => work.await,
    }
}

/// The `Sec-WebSocket-Accept` answer to a request that asks to upgrade to WebSocket, or `None`
/// when it does not (or is not a valid WebSocket request).
pub fn accept_key(headers: &[(String, String)]) -> Option<String> {
    let get = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    let upgrade = get("upgrade")?.eq_ignore_ascii_case("websocket");
    let connection = get("connection")?
        .split(',')
        .any(|t| t.trim().eq_ignore_ascii_case("upgrade"));
    let version = get("sec-websocket-version")?.trim() == "13";
    let key = get("sec-websocket-key")?;
    (upgrade && connection && version).then(|| derive_accept_key(key.trim().as_bytes()))
}

/// The WebSocket on an upgraded server connection.
pub async fn upgraded(io: hyper::upgrade::Upgraded) -> Ws {
    let io: Box<dyn Io> = Box::new(TokioIo::new(io));
    WebSocketStream::from_raw_socket(io, Role::Server, None).await
}

/// The next data message (text or binary); `None` once the peer has closed the connection.
pub async fn recv(ws: &mut Ws, limit: Option<Duration>) -> Result<Option<Message>, String> {
    use futures_util::StreamExt;
    let next = async {
        loop {
            match ws.next().await {
                None | Some(Ok(Message::Close(_))) => return Ok(None),
                Some(Ok(m @ (Message::Text(_) | Message::Binary(_)))) => return Ok(Some(m)),
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(format!("WebSocket: {e}")),
            }
        }
    };
    match limit {
        Some(d) => tokio::time::timeout(d, next)
            .await
            .map_err(|_| "muda umekwisha".to_string())?,
        None => next.await,
    }
}

/// Send one message.
pub async fn send(ws: &mut Ws, m: Message) -> Result<(), String> {
    use futures_util::SinkExt;
    ws.send(m).await.map_err(|e| format!("WebSocket: {e}"))
}

/// Close politely (a close frame, then the socket).
pub async fn close(ws: &mut Ws) {
    use futures_util::SinkExt;
    let _ = ws.close(None).await;
    let _ = ws.flush().await;
}
