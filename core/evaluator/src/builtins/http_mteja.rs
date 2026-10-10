//! HTTP client (`mfumo`): `http_ombi(njia, anwani, chaguo?) -> Tokeo<JibuHttp, Neno>`, the one
//! entry point for every request. `chaguo` is a `ChaguoHttp` (every field optional; the contract
//! is in `core/parser/src/builtins.rs`) covering headers, query, text/base64/JSON/form/multipart/
//! file bodies, Basic and Bearer auth, timeouts, redirects, retries, proxies (HTTP and SOCKS),
//! IPv4/IPv6, custom roots and client certificates, cookies, size limits, streaming the body to a
//! file, and turning a non-2xx status into an error.
//!
//! Built on `asili_mtandao::http::mteja` (hyper: HTTP/1.1 and HTTP/2, rustls with the aws-lc-rs
//! provider the TLS server also uses, the Mozilla roots, gzip and brotli, pooled keep-alive
//! connections). The wait goes through `kazi_sawia::block_on`, so inside a `sawia` task it lets
//! the thread's other tasks run. Redirects, retries and cookies are followed here, so that:
//! - an HTTPS request is never redirected to plain HTTP;
//! - `Authorization`, `Proxy-Authorization` and `Cookie` headers are dropped when a redirect
//!   leaves the origin;
//! - cookies are kept only when asked (`vidakuzi: kweli`), in one jar for the whole program;
//! - a retry re-sends the whole body (a file is reopened).
//!
//! There is no option to turn certificate checking off. Unavailable in the browser build.

use super::http_thamani::{kamusi_to_pairs as text_pairs, unwrap_some};
use super::BuiltinFn;
use crate::value::{self, Value};
use std::collections::HashMap;

/// The response body read into memory, by default (`kikomo` changes it): large enough for any
/// API response, small enough that a hostile server cannot exhaust memory. Use `hifadhi` to
/// stream larger bodies to a file.
const BODY_LIMIT: u64 = 64 * 1024 * 1024;
/// Redirects followed by default (`elekezo`).
const MAX_REDIRECTS: u32 = 10;
/// The whole request, by default (`muda`).
const TIMEOUT_SECS: f64 = 60.0;
/// The longest wait between retries, whatever a server's `Retry-After` asks for.
const MAX_RETRY_WAIT_SECS: f64 = 30.0;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "http_ombi".to_string(),
        Box::new(|args: &[Value]| {
            let method = super::arg_str(args, 0);
            let url = super::arg_str(args, 1);
            Ok(match ombi(&method, &url, args.get(2)) {
                Ok(jibu) => Value::sawa(jibu),
                Err(e) => Value::kosa(format!("http_ombi {method} {url}: {e}")),
            })
        }),
    );
}

/// `ChaguoHttp`, read from the struct value (absent fields are `Hamna`).
#[derive(Default)]
struct Options {
    headers: Vec<(String, String)>,
    query: Vec<(String, String)>,
    body: Option<String>,
    body_base64: Option<String>,
    json: Option<Value>,
    form: Vec<(String, String)>,
    multipart_text: Vec<(String, String)>,
    multipart_files: Vec<(String, String)>,
    file: Option<String>,
    content_type: Option<String>,
    user: Option<String>,
    password: Option<String>,
    token: Option<String>,
    timeout: Option<f64>,
    connect_timeout: Option<f64>,
    redirects: Option<u32>,
    retries: u32,
    proxy: Option<String>,
    ip_family: Option<u8>,
    ca: Option<String>,
    cert: Option<String>,
    key: Option<String>,
    cookies: bool,
    limit: Option<u64>,
    save_to: Option<String>,
    base64_response: bool,
    status_error: bool,
}

impl Options {
    fn from_value(v: Option<&Value>) -> Result<Self, String> {
        let mut o = Options::default();
        let Some(v) = v.map(unwrap_some) else {
            return Ok(o);
        };
        let fields = match v {
            Value::Struct(_, fields) => fields.clone(),
            Value::Hamna | Value::Tupu => return Ok(o),
            _ => return Err("chaguo lazima kiwe ChaguoHttp".into()),
        };
        for (name, value) in fields.iter() {
            let value = unwrap_some(value);
            if matches!(value, Value::Hamna) {
                continue;
            }
            let text =
                || value::as_string(value).ok_or_else(|| format!("uga '{name}' unahitaji Neno"));
            let number = || {
                value::as_f64(value)
                    .filter(|n| n.is_finite() && *n >= 0.0)
                    .ok_or_else(|| format!("uga '{name}' unahitaji Namba isiyo hasi"))
            };
            let truth = || match value {
                Value::Ukweli(b) => Ok(*b),
                _ => Err(format!("uga '{name}' unahitaji Ukweli")),
            };
            let pairs = || {
                text_pairs(value)
                    .ok_or_else(|| format!("uga '{name}' unahitaji Kamusi<Neno, Neno>"))
            };
            match &**name {
                "vichwa" => o.headers = pairs()?,
                "hoja" => o.query = pairs()?,
                "mwili" => o.body = Some(text()?),
                "mwili_base64" => o.body_base64 = Some(text()?),
                "json" => o.json = Some(value.clone()),
                "fomu" => o.form = pairs()?,
                "fomu_sehemu" => o.multipart_text = pairs()?,
                "fomu_faili" => o.multipart_files = pairs()?,
                "faili" => o.file = Some(text()?),
                "aina" => o.content_type = Some(text()?),
                "mtumiaji" => o.user = Some(text()?),
                "nenosiri" => o.password = Some(text()?),
                "tokeni" => o.token = Some(text()?),
                "muda" => o.timeout = Some(number()?),
                "muda_kuunganisha" => o.connect_timeout = Some(number()?),
                "elekezo" => o.redirects = Some(number()?.min(100.0) as u32),
                "jaribu_tena" => o.retries = number()?.min(10.0) as u32,
                "wakala" => o.proxy = Some(text()?),
                "familia_ip" => {
                    o.ip_family = match number()? as u8 {
                        4 => Some(4),
                        6 => Some(6),
                        _ => return Err("familia_ip ni 4 au 6".into()),
                    }
                }
                "cheti_ca" => o.ca = Some(text()?),
                "cheti" => o.cert = Some(text()?),
                "ufunguo" => o.key = Some(text()?),
                "vidakuzi" => o.cookies = truth()?,
                "kikomo" => o.limit = Some(number()? as u64),
                "hifadhi" => o.save_to = Some(text()?),
                "jibu_base64" => o.base64_response = truth()?,
                "kosa_hali" => o.status_error = truth()?,
                other => return Err(format!("ChaguoHttp haina uga '{other}'")),
            }
        }
        let bodies = [
            o.body.is_some(),
            o.body_base64.is_some(),
            o.json.is_some(),
            !o.form.is_empty(),
            !o.multipart_text.is_empty() || !o.multipart_files.is_empty(),
            o.file.is_some(),
        ];
        if bodies.iter().filter(|b| **b).count() > 1 {
            return Err(
                "chagua mwili mmoja tu: mwili, mwili_base64, json, fomu, fomu_sehemu/fomu_faili au faili"
                    .into(),
            );
        }
        if o.cert.is_some() != o.key.is_some() {
            return Err("cheti na ufunguo vinahitajiana".into());
        }
        if o.save_to.is_some() && o.base64_response {
            return Err("hifadhi na jibu_base64 haviwezi kutumika pamoja".into());
        }
        Ok(o)
    }
}

#[cfg(target_arch = "wasm32")]
fn ombi(_method: &str, _url: &str, _options: Option<&Value>) -> Result<Value, String> {
    Err("HTTP haipatikani kwenye kivinjari".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn ombi(method: &str, url: &str, options: Option<&Value>) -> Result<Value, String> {
    let options = Options::from_value(options)?;
    let response = native::send(method, url, &options)?;
    if options.status_error && !(200..300).contains(&response.status) {
        let mut snippet: String = response.body.chars().take(200).collect();
        if snippet.len() < response.body.len() {
            snippet.push('…');
        }
        return Err(format!("HTTP {}: {}", response.status, snippet.trim_end()));
    }
    Ok(super::http_thamani::response_to_value(response))
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::super::http_thamani::Response;
    use super::Options;
    use asili_mtandao::http::mteja as net;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use url::Url;

    /// One jar for every request that asks for cookies.
    fn jar() -> &'static Mutex<cookie_store::CookieStore> {
        static JAR: OnceLock<Mutex<cookie_store::CookieStore>> = OnceLock::new();
        JAR.get_or_init(Default::default)
    }

    /// Everything a request needs, owned, so it can wait on the network.
    struct Plan {
        method: String,
        url: Url,
        headers: Vec<(String, String)>,
        body: net::Body,
        transport: net::Transport,
        cookies: bool,
        retries: u32,
        redirects: u32,
        timeout: f64,
        limit: u64,
        save_to: Option<String>,
        base64_response: bool,
    }

    fn body_of(o: &Options) -> Result<(net::Body, Option<String>), String> {
        use base64::Engine;
        Ok(if let Some(text) = &o.body {
            (
                net::Body::Bytes(text.clone().into_bytes()),
                Some("text/plain; charset=utf-8".into()),
            )
        } else if let Some(b64) = &o.body_base64 {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64.trim())
                .map_err(|e| format!("mwili_base64: {e}"))?;
            (
                net::Body::Bytes(bytes),
                Some("application/octet-stream".into()),
            )
        } else if let Some(v) = &o.json {
            let json = v.to_json().map_err(|e| format!("json: {e}"))?;
            let bytes = serde_json::to_vec(&json).map_err(|e| format!("json: {e}"))?;
            (net::Body::Bytes(bytes), Some("application/json".into()))
        } else if !o.form.is_empty() {
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(&o.form)
                .finish();
            (
                net::Body::Bytes(encoded.into_bytes()),
                Some("application/x-www-form-urlencoded".into()),
            )
        } else if !o.multipart_text.is_empty() || !o.multipart_files.is_empty() {
            let (kind, bytes) = net::multipart(&o.multipart_text, &o.multipart_files)?;
            (net::Body::Bytes(bytes), Some(kind))
        } else if let Some(path) = &o.file {
            (
                net::Body::File(path.clone()),
                Some("application/octet-stream".into()),
            )
        } else {
            (net::Body::Empty, None)
        })
    }

    /// `Retry-After` in seconds (only the delay form; a date waits the default).
    fn retry_after(headers: &[(String, String)]) -> Option<f64> {
        headers
            .iter()
            .find(|(k, _)| k == "retry-after")?
            .1
            .trim()
            .parse::<f64>()
            .ok()
    }

    pub(super) fn send(method: &str, url: &str, o: &Options) -> Result<Response, String> {
        let mut url = Url::parse(url.trim()).map_err(|e| format!("anwani si sahihi: {e}"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err("anwani lazima ianze na http:// au https://".into());
        }
        if !o.query.is_empty() {
            url.query_pairs_mut().extend_pairs(&o.query);
        }
        let (body, body_type) = body_of(o)?;
        let mut headers = o.headers.clone();
        if let Some(user) = &o.user {
            use base64::Engine;
            let pair = format!("{user}:{}", o.password.as_deref().unwrap_or(""));
            let encoded = base64::engine::general_purpose::STANDARD.encode(pair);
            headers.push(("authorization".into(), format!("Basic {encoded}")));
        } else if let Some(token) = &o.token {
            headers.push(("authorization".into(), format!("Bearer {token}")));
        }
        let has_type = headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
        if let Some(t) = o.content_type.clone().or(body_type).filter(|_| !has_type) {
            headers.push(("content-type".into(), t));
        }
        let plan = Plan {
            method: method.trim().to_ascii_uppercase(),
            url,
            headers,
            body,
            transport: net::Transport {
                proxy: o.proxy.clone(),
                ip_family: o.ip_family,
                ca_file: o.ca.clone(),
                client_cert: o.cert.clone().zip(o.key.clone()),
                connect_timeout_ms: o
                    .connect_timeout
                    .filter(|s| *s > 0.0)
                    .map(|s| (s * 1000.0) as u64),
            },
            cookies: o.cookies,
            retries: o.retries,
            redirects: o.redirects.unwrap_or(super::MAX_REDIRECTS),
            timeout: o.timeout.unwrap_or(super::TIMEOUT_SECS),
            limit: o.limit.unwrap_or(super::BODY_LIMIT),
            save_to: o.save_to.clone(),
            base64_response: o.base64_response,
        };
        crate::platform::flush_stdout(); // about to wait: show what was printed so far
        crate::kazi_sawia::block_on(async move { run(plan).await }).map_err(|e| format!("{e:?}"))?
    }

    async fn run(plan: Plan) -> Result<Response, String> {
        let timeout = plan.timeout;
        let work = exchange(plan);
        if timeout > 0.0 {
            tokio::time::timeout(Duration::from_secs_f64(timeout), work)
                .await
                .map_err(|_| "muda umekwisha".to_string())?
        } else {
            work.await
        }
    }

    async fn exchange(mut p: Plan) -> Result<Response, String> {
        let started = Instant::now();
        let mut redirects = 0;
        let mut attempt = 0;
        let (response, final_url) = loop {
            let mut headers = p.headers.clone();
            if p.cookies {
                let jar = jar().lock().unwrap_or_else(|e| e.into_inner());
                let cookie = jar
                    .get_request_values(&p.url)
                    .map(|(n, v)| format!("{n}={v}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                if !cookie.is_empty() {
                    headers.push(("cookie".into(), cookie));
                }
            }
            let request = net::Request {
                method: &p.method,
                url: &p.url,
                headers: &headers,
                body: &p.body,
            };
            let result = match net::send(&p.transport, &request).await {
                Ok(r) => r,
                // A connection that failed is retried like a 503.
                Err(_) if attempt < p.retries && idempotent(&p.method) => {
                    attempt += 1;
                    tokio::time::sleep(backoff(attempt, None)).await;
                    continue;
                }
                Err(e) => return Err(e),
            };
            if p.cookies {
                let set = result
                    .headers
                    .iter()
                    .filter(|(k, _)| k == "set-cookie")
                    .filter_map(|(_, v)| cookie_store::RawCookie::parse(v.clone()).ok())
                    .collect::<Vec<_>>();
                jar()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .store_response_cookies(set.into_iter(), &p.url);
            }
            let status = result.status;
            if matches!(status, 429 | 502 | 503 | 504)
                && attempt < p.retries
                && idempotent(&p.method)
            {
                attempt += 1;
                tokio::time::sleep(backoff(attempt, retry_after(&result.headers))).await;
                continue;
            }
            let location = result
                .headers
                .iter()
                .find(|(k, _)| k == "location")
                .map(|(_, v)| v.clone());
            match (status, location) {
                (301 | 302 | 303 | 307 | 308, Some(location)) if redirects < p.redirects => {
                    let next = p
                        .url
                        .join(&location)
                        .map_err(|e| format!("kuelekezwa kwenda '{location}': {e}"))?;
                    if p.url.scheme() == "https" && next.scheme() != "https" {
                        return Err(format!(
                            "kuelekezwa kutoka HTTPS kwenda {} kumekataliwa",
                            next.scheme()
                        ));
                    }
                    if !matches!(next.scheme(), "http" | "https") {
                        return Err(format!("kuelekezwa kwenda '{next}' kumekataliwa"));
                    }
                    if next.origin() != p.url.origin() {
                        p.headers.retain(|(k, _)| {
                            !["authorization", "proxy-authorization", "cookie"]
                                .iter()
                                .any(|h| k.eq_ignore_ascii_case(h))
                        });
                    }
                    // 303, and 301/302 after a POST, continue as a GET without the body.
                    if status == 303 || (matches!(status, 301 | 302) && p.method == "POST") {
                        if p.method != "HEAD" {
                            p.method = "GET".into();
                        }
                        p.body = net::Body::Empty;
                        p.headers
                            .retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
                    }
                    redirects += 1;
                    p.url = next;
                }
                _ => break (result, p.url.clone()),
            }
        };
        let (status, reason, version) =
            (response.status, response.reason.clone(), response.version);
        let resp_headers = response.headers.clone();
        let charset = resp_headers
            .iter()
            .find(|(k, _)| k == "content-type")
            .and_then(|(_, t)| {
                t.split(';')
                    .filter_map(|part| part.trim().split_once('='))
                    .find(|(k, _)| k.trim().eq_ignore_ascii_case("charset"))
                    .map(|(_, v)| v.trim().trim_matches('"').to_string())
            });
        let body = if let Some(path) = &p.save_to {
            save(response, p.limit, path).await?;
            String::new()
        } else {
            let mut bytes = Vec::new();
            response
                .read_body(p.limit, &mut |c| {
                    bytes.extend_from_slice(c);
                    Ok(())
                })
                .await?;
            if p.base64_response {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(bytes)
            } else {
                decode(bytes, charset.as_deref())
            }
        };
        Ok(Response {
            status,
            reason,
            version: version.to_string(),
            headers: resp_headers,
            body,
            url: final_url.to_string(),
            seconds: started.elapsed().as_secs_f64(),
        })
    }

    /// Stream the body to `path`, through a temporary file renamed into place when complete,
    /// so a failed download never leaves a partial file under the requested name.
    async fn save(response: net::Response, limit: u64, path: &str) -> Result<(), String> {
        let tmp = format!("{path}.sehemu");
        let result = async {
            let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
            response
                .read_body(limit, &mut |c| file.write_all(c))
                .await?;
            file.sync_all().map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, path).map_err(|e| e.to_string())
        }
        .await;
        result.map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("hifadhi {path}: {e}")
        })
    }

    /// The body as text in its declared charset (UTF-8 when none); invalid bytes become U+FFFD.
    fn decode(bytes: Vec<u8>, charset: Option<&str>) -> String {
        if let Some(enc) = charset.and_then(|c| encoding_rs::Encoding::for_label(c.as_bytes())) {
            if enc != encoding_rs::UTF_8 {
                return enc.decode(&bytes).0.into_owned();
            }
        }
        match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
        }
    }

    fn idempotent(m: &str) -> bool {
        matches!(m, "GET" | "HEAD" | "PUT" | "DELETE" | "OPTIONS" | "TRACE")
    }

    /// 0.5 s, 1 s, 2 s, … (or the server's `Retry-After`), never more than 30 s.
    fn backoff(attempt: u32, server: Option<f64>) -> Duration {
        let wait = server.unwrap_or(0.25 * 2f64.powi(attempt as i32));
        Duration::from_secs_f64(wait.clamp(0.0, super::MAX_RETRY_WAIT_SECS))
    }
}
