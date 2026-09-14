//! HTTP/1.1 request/response framing on top of a `Mkondo` stream. See
//! docs/design/http-framing-design.md for the `OmbiHttp`/`JibuHttp` `Struct`-reuse decision, the
//! `.soma_bailisi` primitive this is built on, and the chunked/pipelining/100-continue scope
//! boundaries (all explicitly out of scope for this pass — fixed `Content-Length` only, no
//! speculative reads ahead of one request/response cycle, no `100 Continue` intermediate).
//!
//! `mkondo_tumikia_http` is the framed counterpart to `mkondo_tumikia` (Phase 11's raw-bytes
//! listener) — a separate, additive entry point, not a mode flag: `kazi_jina`'s contract here is
//! `kazi_jina(ombi: OmbiHttp) -> JibuHttp`, not `kazi_jina(mkondo: Mkondo) -> Tupu`. The
//! raw-bytes `mkondo_tumikia` contract (examples/mkondo_server/) is completely untouched by this
//! module.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;

use crate::value::{self, EvalError, MapKey, MkondoStream, Value};

#[cfg(not(target_arch = "wasm32"))]
use super::mkondo::{apply_connection_timeout, handshake_tls};

/// How many bytes `.soma_bailisi`-equivalent reads pull at a time while accumulating a request.
/// Not user-configurable in this pass — matches `CONNECTION_TIMEOUT`'s "fixed for now, a natural
/// follow-up once real usage data exists" scoping in `mkondo.rs`.
const READ_CHUNK: usize = 8192;
/// Hard cap on total request size (headers + body) accepted before giving up and returning a 400
/// — an unbounded accumulation buffer fed by an adversarial peer is exactly the kind of
/// resource-exhaustion risk the original production-readiness survey flagged.
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_HEADERS: usize = 64;

struct ParsedRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

enum ParseOutcome {
    Ok(ParsedRequest, bool /* keep_alive */),
    /// Peer closed the connection before sending a complete request — not an error, just "no
    /// more requests on this connection." Distinguishes a clean keep-alive-loop exit from an
    /// actual malformed-request 400.
    ConnectionClosed,
    BadRequest(String),
    NotImplemented(String),
}

/// Reads and parses exactly one HTTP/1.1 (or 1.0) request off `stream`, using repeated bounded
/// reads (the same primitive `.soma_bailisi` exposes at the Asili level) rather than
/// `.soma()`'s read-to-EOF, which would make keep-alive impossible — the peer isn't expected to
/// close the connection between requests.
fn read_request(stream: &mut MkondoStream) -> ParseOutcome {
    let mut buf: Vec<u8> = Vec::new();
    let mut header_end: Option<usize> = None;

    loop {
        if buf.len() >= MAX_REQUEST_BYTES {
            return ParseOutcome::BadRequest("ombi ni kubwa mno".to_string());
        }

        if header_end.is_none() {
            let mut headers_storage = [httparse::EMPTY_HEADER; MAX_HEADERS];
            let mut req = httparse::Request::new(&mut headers_storage);
            match req.parse(&buf) {
                Ok(httparse::Status::Complete(offset)) => header_end = Some(offset),
                Ok(httparse::Status::Partial) => {}
                Err(_) => return ParseOutcome::BadRequest("mstari wa ombi au vichwa batili".to_string()),
            }
        }

        if let Some(offset) = header_end {
            // Re-parse now that we know headers are complete, to read method/path/headers
            // (borrow-checker reasons: `req` above borrows `buf`, which we need to mutate in
            // the read loop below, so it can't be kept alive across iterations).
            let mut headers_storage = [httparse::EMPTY_HEADER; MAX_HEADERS];
            let mut req = httparse::Request::new(&mut headers_storage);
            let _ = req.parse(&buf);

            let method = req.method.unwrap_or("").to_string();
            let path = req.path.unwrap_or("").to_string();
            let version = req.version.unwrap_or(1);
            let headers: Vec<(String, String)> = req
                .headers
                .iter()
                .map(|h| (h.name.to_string(), String::from_utf8_lossy(h.value).into_owned()))
                .collect();

            let content_length = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, v)| v.trim().parse::<usize>().ok());
            let is_chunked = headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("transfer-encoding") && v.to_ascii_lowercase().contains("chunked"));
            if is_chunked {
                return ParseOutcome::NotImplemented("Transfer-Encoding: chunked haiungwi mkono".to_string());
            }

            let body_len = content_length.unwrap_or(0);
            let needed = offset + body_len;
            if buf.len() >= needed {
                let body = String::from_utf8_lossy(&buf[offset..needed]).into_owned();
                let keep_alive = connection_keep_alive(&headers, version);
                return ParseOutcome::Ok(ParsedRequest { method, path, headers, body }, keep_alive);
            }
            // else: headers complete but body not fully received yet — fall through to read more.
        }

        let mut chunk = vec![0u8; READ_CHUNK];
        match stream.read(&mut chunk) {
            Ok(0) => {
                return if buf.is_empty() {
                    ParseOutcome::ConnectionClosed
                } else {
                    ParseOutcome::BadRequest("muunganisho umefungwa kabla ombi kukamilika".to_string())
                };
            }
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                return ParseOutcome::BadRequest("muda wa kusoma umeisha".to_string());
            }
            Err(e) => return ParseOutcome::BadRequest(format!("hitilafu ya kusoma: {e}")),
        }
    }
}

/// HTTP/1.1 defaults to keep-alive unless `Connection: close` is present; HTTP/1.0 defaults to
/// close unless `Connection: keep-alive` is explicitly present — the inverse default, per spec.
fn connection_keep_alive(headers: &[(String, String)], version: u8) -> bool {
    let connection = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("connection"))
        .map(|(_, v)| v.to_ascii_lowercase());
    match connection {
        Some(v) if v.contains("close") => false,
        Some(v) if v.contains("keep-alive") => true,
        _ => version >= 1,
    }
}

fn write_response(stream: &mut MkondoStream, status: u16, headers: &[(String, String)], body: &str) -> std::io::Result<()> {
    let reason = status_reason(status);
    let mut out = format!("HTTP/1.1 {status} {reason}\r\n");
    let mut has_content_length = false;
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        }
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    if !has_content_length {
        out.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    out.push_str("\r\n");
    out.push_str(body);
    stream.write_all(out.as_bytes())
}

fn status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "",
    }
}

/// `OmbiHttp` -> `Value::Struct` fields: njia, anwani, vichwa, mwili. Reflection-friendly shape
/// already established by the JSON codec (`Value::Struct` is name+field-list, no new runtime
/// type needed) — `httparse`'s parsed method/path/headers map onto it directly.
fn request_to_value(req: &ParsedRequest) -> Value {
    let mut headers_map: HashMap<MapKey, Value> = HashMap::with_capacity(req.headers.len());
    for (k, v) in &req.headers {
        headers_map.insert(MapKey::Neno(k.clone()), Value::Neno(v.clone()));
    }
    Value::Struct(
        "OmbiHttp".to_string(),
        vec![
            ("njia".to_string(), Value::Neno(req.method.clone())),
            ("anwani".to_string(), Value::Neno(req.path.clone())),
            ("vichwa".to_string(), Value::Kamusi(headers_map)),
            ("mwili".to_string(), Value::Neno(req.body.clone())),
        ],
    )
}

/// `JibuHttp` (`Value::Struct("JibuHttp", [("hali", Namba), ("vichwa", Kamusi), ("mwili", Neno)])`)
/// -> the pieces `write_response` needs. `kazi_jina`'s own return value doesn't need to be this
/// exact struct — anything with the right field names/types works, since `Value::Struct`'s name
/// is never checked here, matching the JSON codec's own struct-shape leniency.
fn value_to_response(v: &Value) -> Option<(u16, Vec<(String, String)>, String)> {
    let Value::Struct(_, fields) = v else { return None };
    let hali = fields.iter().find(|(n, _)| n == "hali").and_then(|(_, v)| value::as_f64(v))?;
    let mwili = fields
        .iter()
        .find(|(n, _)| n == "mwili")
        .and_then(|(_, v)| value::as_string(v))
        .unwrap_or_default();
    let vichwa = fields
        .iter()
        .find(|(n, _)| n == "vichwa")
        .map(|(_, v)| match v {
            Value::Kamusi(m) => m
                .iter()
                .filter_map(|(k, v)| match (k, value::as_string(v)) {
                    (MapKey::Neno(k), Some(v)) => Some((k.clone(), v)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default();
    Some((hali as u16, vichwa, mwili))
}

/// `mkondo_tumikia_http(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls: Chaguo<TlsUsanidi>) ->
/// Tokeo<Tupu, Neno>` — same worker-pool/timeout/TLS-branch structure as `mkondo_tumikia`
/// (`mkondo.rs`), but its per-connection loop parses one HTTP request, calls
/// `kazi_jina(ombi: OmbiHttp) -> JibuHttp`, writes a real HTTP/1.1 response, and — unlike the
/// raw-bytes `mkondo_tumikia`, which handles exactly one request per accepted connection —
/// loops to parse the *next* request on the same connection when the client asked for
/// keep-alive, closing only on `Connection: close`, a parse error, or the peer disconnecting.
pub(crate) fn mkondo_tumikia_http(
    module: &asili_parser::Module,
    args: &[Value],
) -> Result<Value, EvalError> {
    let listener = match args.first() {
        Some(Value::MkondoSikilizaji(l)) => Arc::clone(l),
        _ => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "mkondo_tumikia_http: hoja ya kwanza lazima iwe MkondoSikilizaji".to_string(),
            )))))
        }
    };
    let kazi_name = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
    if !module.functions.iter().any(|f| f.name == kazi_name) {
        return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
            "mkondo_tumikia_http: kazi haijulikani: {kazi_name}"
        ))))));
    }
    let idadi_ya_nyuzi = value::as_f64(args.get(2).unwrap_or(&Value::Hamna))
        .unwrap_or(0.0)
        .max(1.0) as usize;
    // Same Chaguo<T> dual-representation handling mkondo_tumikia needs — see
    // docs/design/tls-design.md's "Chaguo<T> dual-representation trap" section.
    #[cfg(not(target_arch = "wasm32"))]
    let tls_inner: Option<&Value> = match args.get(3) {
        Some(Value::Chaguo(Some(inner))) => Some(inner.as_ref()),
        Some(Value::Enum(en, vn, Some(inner))) if en == "Chaguo" && vn == "Kuna" => Some(inner.as_ref()),
        Some(Value::Chaguo(None)) => None,
        Some(Value::Enum(en, vn, None)) if en == "Chaguo" && vn == "Hamna" => None,
        None | Some(Value::Hamna) => None,
        Some(_) => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "mkondo_tumikia_http: hoja ya nne (tls) lazima iwe Chaguo<TlsUsanidi>".to_string(),
            )))))
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    let tls_config: Option<Arc<rustls::ServerConfig>> = match tls_inner {
        Some(Value::TlsUsanidi(cfg)) => Some(Arc::clone(cfg)),
        Some(_) => {
            return Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "mkondo_tumikia_http: hoja ya nne (tls) lazima iwe Chaguo<TlsUsanidi>".to_string(),
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
            http_worker_loop(&listener, &module_owned, &kazi_name, tls_config);
            #[cfg(target_arch = "wasm32")]
            http_worker_loop(&listener, &module_owned, &kazi_name);
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    Ok(Value::Tokeo(Ok(Box::new(Value::Tupu))))
}

fn http_worker_loop(
    listener: &std::net::TcpListener,
    module: &asili_parser::Module,
    kazi_name: &str,
    #[cfg(not(target_arch = "wasm32"))] tls_config: Option<Arc<rustls::ServerConfig>>,
) {
    loop {
        let tcp_stream = match listener.accept() {
            Ok((s, _addr)) => s,
            Err(_) => return,
        };
        #[cfg(not(target_arch = "wasm32"))]
        apply_connection_timeout(&tcp_stream);

        #[cfg(not(target_arch = "wasm32"))]
        let mut mkondo_stream = match &tls_config {
            Some(cfg) => match handshake_tls(Arc::clone(cfg), tcp_stream) {
                Some(s) => s,
                None => continue,
            },
            None => MkondoStream::Wazi(tcp_stream),
        };
        #[cfg(target_arch = "wasm32")]
        let mut mkondo_stream = MkondoStream::Wazi(tcp_stream);

        // Keep-alive loop: parse and answer requests on this one connection until the client
        // asks to close, a parse error occurs, or the peer disconnects.
        loop {
            let outcome = read_request(&mut mkondo_stream);
            let (parsed, keep_alive) = match outcome {
                ParseOutcome::Ok(req, ka) => (req, ka),
                ParseOutcome::ConnectionClosed => break,
                ParseOutcome::BadRequest(msg) => {
                    let _ = write_response(&mut mkondo_stream, 400, &[], &msg);
                    break;
                }
                ParseOutcome::NotImplemented(msg) => {
                    let _ = write_response(&mut mkondo_stream, 501, &[], &msg);
                    break;
                }
            };

            let ombi = request_to_value(&parsed);
            let jibu_result = crate::run_function(module, kazi_name, vec![ombi]);
            let write_ok = match jibu_result {
                Ok(jibu_val) => match value_to_response(&jibu_val) {
                    Some((status, headers, body)) => write_response(&mut mkondo_stream, status, &headers, &body).is_ok(),
                    None => write_response(&mut mkondo_stream, 500, &[], "jibu batili kutoka kwa kazi_jina").is_ok(),
                },
                Err(_) => write_response(&mut mkondo_stream, 500, &[], "hitilafu ya ndani").is_ok(),
            };

            if !write_ok || !keep_alive {
                break;
            }
        }
        // mkondo_stream drops here — MkondoStream's own Drop impl sends TLS close_notify if
        // this was a Salama connection, same as the raw-bytes mkondo_tumikia path.
    }
}
