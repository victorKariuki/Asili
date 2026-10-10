//! HTTP/3 (RFC 9114) over QUIC (quinn): a server that answers on UDP beside the TCP listener, and
//! a client for one request. Both use the TLS configuration of the TCP side (`asili_mtandao::tls`),
//! with the `h3` protocol name.

use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::Arc;

use bytes::{Buf, Bytes};
use http_body_util::BodyExt;

use super::seva::{self, Handler, Jibu, ServerOptions};
use crate::tls;

/// Serve HTTP/3 on `socket` until the returned endpoint is closed. Call inside a `LocalSet`.
pub fn start(
    socket: UdpSocket,
    tls_config: Arc<rustls::ServerConfig>,
    handler: Handler,
    opts: ServerOptions,
) -> Result<quinn::Endpoint, String> {
    let mut cfg = (*tls_config).clone();
    cfg.alpn_protocols = vec![b"h3".to_vec()];
    let qc = quinn::crypto::rustls::QuicServerConfig::try_from(cfg).map_err(|e| e.to_string())?;
    let server = quinn::ServerConfig::with_crypto(Arc::new(qc));
    let endpoint = quinn::Endpoint::new(
        quinn::EndpointConfig::default(),
        Some(server),
        socket,
        Arc::new(quinn::TokioRuntime),
    )
    .map_err(|e| format!("HTTP/3: {e}"))?;
    let ep = endpoint.clone();
    tokio::task::spawn_local(async move {
        while let Some(incoming) = ep.accept().await {
            let (handler, opts) = (handler.clone(), opts.clone());
            tokio::task::spawn_local(async move {
                let Ok(conn) = incoming.await else { return };
                let peer = conn.remote_address().to_string();
                let Ok(mut h3) = h3::server::builder()
                    .build(h3_quinn::Connection::new(conn))
                    .await
                else {
                    return;
                };
                while let Ok(Some(resolver)) = h3.accept().await {
                    let (handler, opts, peer) = (handler.clone(), opts.clone(), peer.clone());
                    tokio::task::spawn_local(async move {
                        let _ = answer(resolver, handler, opts, peer).await;
                    });
                }
            });
        }
    });
    Ok(endpoint)
}

/// One request on an HTTP/3 connection: its body, the handler's answer, and the answer sent.
async fn answer(
    resolver: h3::server::RequestResolver<h3_quinn::Connection, Bytes>,
    handler: Handler,
    opts: ServerOptions,
    peer: String,
) -> Result<(), String> {
    let (req, mut stream) = resolver
        .resolve_request()
        .await
        .map_err(|e| e.to_string())?;
    let (parts, ()) = req.into_parts();
    let mut body = Vec::new();
    while let Some(mut chunk) = stream.recv_data().await.map_err(|e| e.to_string())? {
        while chunk.has_remaining() {
            let piece = chunk.chunk();
            if body.len() + piece.len() > opts.body_limit {
                return send_plain(&mut stream, 413, "mwili wa ombi ni mkubwa mno").await;
            }
            body.extend_from_slice(piece);
            let n = piece.len();
            chunk.advance(n);
        }
    }
    let ombi = seva::ombi_of(&parts, body, peer);
    let jibu: Jibu = handler(ombi).await;
    if jibu.websocket.is_some() {
        return send_plain(&mut stream, 400, "WebSocket haipo kwenye HTTP/3").await;
    }
    let accept = parts
        .headers
        .get(hyper::header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let (head, mut data) = seva::answer(jibu, &accept, opts.compress).into_parts();
    let mut out = hyper::http::Response::builder().status(head.status);
    for (k, v) in head.headers.iter() {
        out = out.header(k, v);
    }
    stream
        .send_response(out.body(()).map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?;
    while let Some(Ok(frame)) = data.frame().await {
        if let Ok(piece) = frame.into_data() {
            if !piece.is_empty() {
                stream.send_data(piece).await.map_err(|e| e.to_string())?;
            }
        }
    }
    stream.finish().await.map_err(|e| e.to_string())
}

async fn send_plain(
    stream: &mut h3::server::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    status: u16,
    msg: &str,
) -> Result<(), String> {
    let resp = hyper::http::Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(())
        .map_err(|e| e.to_string())?;
    stream
        .send_response(resp)
        .await
        .map_err(|e| e.to_string())?;
    stream
        .send_data(Bytes::from(msg.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    stream.finish().await.map_err(|e| e.to_string())
}

/// An HTTP/3 answer, its body decoded (gzip, brotli) and read to the limit.
#[derive(Debug)]
pub struct Answer {
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Send one request to `url` over HTTP/3 and read the answer. `ca_file` as for `tls::connect`.
pub async fn fetch(
    method: &str,
    url: &url::Url,
    headers: &[(String, String)],
    body: Vec<u8>,
    ca_file: Option<&str>,
    limit: u64,
) -> Result<Answer, String> {
    let host = url
        .host_str()
        .ok_or("anwani haina mwenyeji")?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let port = url.port_or_known_default().ok_or("anwani haina mlango")?;
    let addr = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|e| format!("kutafuta {host}: {e}"))?
        .next()
        .ok_or_else(|| format!("kutafuta {host}: hakuna anwani"))?;
    let client_tls = tls::client_config(ca_file, None, &[b"h3".to_vec()])?;
    let qc =
        quinn::crypto::rustls::QuicClientConfig::try_from(client_tls).map_err(|e| e.to_string())?;
    let bind = if addr.is_ipv6() {
        SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)
    } else {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
    };
    let mut endpoint = quinn::Endpoint::client(bind).map_err(|e| format!("HTTP/3: {e}"))?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(qc)));
    let conn = endpoint
        .connect(addr, &host)
        .map_err(|e| format!("HTTP/3: {e}"))?
        .await
        .map_err(|e| format!("HTTP/3: {e}"))?;
    let quic = conn.clone();
    let (mut driver, mut send) = h3::client::new(h3_quinn::Connection::new(conn))
        .await
        .map_err(|e| format!("HTTP/3: {e}"))?;
    tokio::task::spawn_local(async move {
        let _ = std::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    let mut req = hyper::http::Request::builder()
        .method(method)
        .uri(url.as_str());
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let mut stream = send
        .send_request(req.body(()).map_err(|e| e.to_string())?)
        .await
        .map_err(|e| format!("HTTP/3: {e}"))?;
    if !body.is_empty() {
        stream
            .send_data(Bytes::from(body))
            .await
            .map_err(|e| format!("HTTP/3: {e}"))?;
    }
    stream.finish().await.map_err(|e| format!("HTTP/3: {e}"))?;
    let resp = stream
        .recv_response()
        .await
        .map_err(|e| format!("HTTP/3: {e}"))?;
    let status = resp.status().as_u16();
    let resp_headers = super::header_pairs(resp.headers());
    let encoding = resp_headers
        .iter()
        .find(|(k, _)| k == "content-encoding")
        .map(|(_, v)| v.trim().to_ascii_lowercase());
    let mut raw = Vec::new();
    while let Some(mut chunk) = stream
        .recv_data()
        .await
        .map_err(|e| format!("HTTP/3: {e}"))?
    {
        while chunk.has_remaining() {
            let piece = chunk.chunk();
            if raw.len() + piece.len() > limit as usize {
                return Err(format!("jibu limezidi kikomo cha baiti {limit}"));
            }
            raw.extend_from_slice(piece);
            let n = piece.len();
            chunk.advance(n);
        }
    }
    quic.close(0u32.into(), b"done");
    let body = decode(encoding.as_deref(), raw, limit)?;
    let reason = hyper::StatusCode::from_u16(status)
        .ok()
        .and_then(|s| s.canonical_reason())
        .unwrap_or("")
        .to_string();
    Ok(Answer {
        status,
        reason,
        headers: resp_headers,
        body,
    })
}

/// The body with its content coding undone, at most `limit` bytes.
fn decode(encoding: Option<&str>, raw: Vec<u8>, limit: u64) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let read = match encoding {
        Some("gzip" | "x-gzip") => flate2::read::GzDecoder::new(&raw[..])
            .take(limit + 1)
            .read_to_end(&mut out),
        Some("br") => brotli::Decompressor::new(&raw[..], 4096)
            .take(limit + 1)
            .read_to_end(&mut out),
        _ => return Ok(raw),
    };
    read.map_err(|e| format!("kusoma jibu: {e}"))?;
    if out.len() as u64 > limit {
        return Err(format!("jibu limezidi kikomo cha baiti {limit}"));
    }
    Ok(out)
}
