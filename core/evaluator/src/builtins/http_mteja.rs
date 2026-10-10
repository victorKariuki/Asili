//! HTTP client (`mfumo`): `http_ombi(njia, anwani, chaguo?) -> Tokeo<JibuHttp, Neno>`, the one
//! entry point for every request. `chaguo` is a `ChaguoHttp` (every field optional; the contract
//! is in `core/parser/src/builtins.rs`) covering headers, query, text/base64/JSON/form/multipart/
//! file bodies, Basic and Bearer auth, timeouts, redirects, retries, proxies (HTTP and SOCKS),
//! IPv4/IPv6, custom roots and client certificates, cookies, size limits, streaming the body to a
//! file, and turning a non-2xx status into an error.
//!
//! Built on `ureq` (blocking, rustls with the aws-lc-rs provider the TLS server also uses, the
//! Mozilla roots, gzip and brotli). Agents — each holding a keep-alive connection pool — are
//! shared by every request with the same transport settings (proxy, roots, client certificate,
//! IP family), so a program reuses connections across calls and threads. Redirects, retries and
//! cookies are followed here rather than inside `ureq`, so that:
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
    use std::io::Read;
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use ureq::http::{self, header, HeaderName, HeaderValue, Method};
    use url::Url;

    /// What an agent is built from; requests with the same key share its connection pool.
    #[derive(Clone, PartialEq, Eq, Hash)]
    struct AgentKey {
        proxy: Option<String>,
        ip_family: Option<u8>,
        ca: Option<String>,
        cert: Option<(String, String)>,
    }

    fn agent(key: &AgentKey) -> Result<ureq::Agent, String> {
        static AGENTS: OnceLock<Mutex<std::collections::HashMap<AgentKey, ureq::Agent>>> =
            OnceLock::new();
        let agents = AGENTS.get_or_init(Default::default);
        if let Some(a) = agents.lock().unwrap_or_else(|e| e.into_inner()).get(key) {
            return Ok(a.clone());
        }
        let built = build_agent(key)?;
        Ok(agents
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(key.clone())
            .or_insert(built)
            .clone())
    }

    fn build_agent(key: &AgentKey) -> Result<ureq::Agent, String> {
        use ureq::tls::{Certificate, ClientCert, PrivateKey, RootCerts, TlsConfig, TlsProvider};
        // The crypto provider the TLS server uses (aws-lc-rs), so the program holds one.
        let crypto = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let mut tls = TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .unversioned_rustls_crypto_provider(crypto);
        if let Some(path) = &key.ca {
            let pem = std::fs::read(path).map_err(|e| format!("cheti_ca {path}: {e}"))?;
            let certs = ureq::tls::parse_pem(&pem)
                .filter_map(|item| match item {
                    Ok(ureq::tls::PemItem::Certificate(c)) => Some(c),
                    _ => None,
                })
                .collect::<Vec<Certificate<'static>>>();
            if certs.is_empty() {
                return Err(format!("cheti_ca {path}: hakuna cheti cha PEM"));
            }
            tls = tls.root_certs(RootCerts::new_with_certs(&certs));
        }
        if let Some((cert, key)) = &key.cert {
            let pem = std::fs::read(cert).map_err(|e| format!("cheti {cert}: {e}"))?;
            let chain = ureq::tls::parse_pem(&pem)
                .filter_map(|item| match item {
                    Ok(ureq::tls::PemItem::Certificate(c)) => Some(c),
                    _ => None,
                })
                .collect::<Vec<Certificate<'static>>>();
            let key_pem = std::fs::read(key).map_err(|e| format!("ufunguo {key}: {e}"))?;
            let key = PrivateKey::from_pem(&key_pem).map_err(|e| format!("ufunguo {key}: {e}"))?;
            tls = tls.client_cert(Some(ClientCert::new_with_certs(&chain, key)));
        }
        let proxy = match &key.proxy {
            // `wakala: ""` turns the environment's proxy off.
            Some(p) if p.is_empty() => None,
            Some(p) => Some(ureq::Proxy::new(p).map_err(|e| format!("wakala {p}: {e}"))?),
            None => ureq::Proxy::try_from_env(),
        };
        let ip_family = match key.ip_family {
            Some(4) => ureq::config::IpFamily::Ipv4Only,
            Some(6) => ureq::config::IpFamily::Ipv6Only,
            _ => ureq::config::IpFamily::Any,
        };
        Ok(ureq::Agent::config_builder()
            .tls_config(tls.build())
            .proxy(proxy)
            .ip_family(ip_family)
            .http_status_as_error(false)
            // Redirects are followed in `send`.
            .max_redirects(0)
            .max_redirects_will_error(false)
            .allow_non_standard_methods(true)
            .user_agent(concat!("asili/", env!("CARGO_PKG_VERSION")))
            .max_idle_connections(100)
            .max_idle_connections_per_host(10)
            .build()
            .into())
    }

    /// One jar for every request that asks for cookies.
    fn jar() -> &'static Mutex<cookie_store::CookieStore> {
        static JAR: OnceLock<Mutex<cookie_store::CookieStore>> = OnceLock::new();
        JAR.get_or_init(Default::default)
    }

    /// The request body, rebuilt for every attempt.
    enum Body {
        Empty,
        Bytes(Vec<u8>, Option<&'static str>),
        File(String),
        Multipart,
    }

    fn body_of(o: &Options) -> Result<Body, String> {
        use base64::Engine;
        Ok(if let Some(text) = &o.body {
            Body::Bytes(text.clone().into_bytes(), Some("text/plain; charset=utf-8"))
        } else if let Some(b64) = &o.body_base64 {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64.trim())
                .map_err(|e| format!("mwili_base64: {e}"))?;
            Body::Bytes(bytes, Some("application/octet-stream"))
        } else if let Some(v) = &o.json {
            let json = v.to_json().map_err(|e| format!("json: {e}"))?;
            let bytes = serde_json::to_vec(&json).map_err(|e| format!("json: {e}"))?;
            Body::Bytes(bytes, Some("application/json"))
        } else if !o.form.is_empty() {
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(&o.form)
                .finish();
            Body::Bytes(
                encoded.into_bytes(),
                Some("application/x-www-form-urlencoded"),
            )
        } else if !o.multipart_text.is_empty() || !o.multipart_files.is_empty() {
            Body::Multipart
        } else if let Some(path) = &o.file {
            Body::File(path.clone())
        } else {
            Body::Empty
        })
    }

    /// `Retry-After` in seconds (only the delay form; a date waits the default).
    fn retry_after(r: &http::Response<ureq::Body>) -> Option<f64> {
        r.headers()
            .get(header::RETRY_AFTER)?
            .to_str()
            .ok()?
            .trim()
            .parse::<f64>()
            .ok()
    }

    pub(super) fn send(method: &str, url: &str, o: &Options) -> Result<Response, String> {
        let started = Instant::now();
        let mut method = Method::from_bytes(method.trim().to_ascii_uppercase().as_bytes())
            .map_err(|_| "njia si sahihi".to_string())?;
        let mut url = Url::parse(url.trim()).map_err(|e| format!("anwani si sahihi: {e}"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err("anwani lazima ianze na http:// au https://".into());
        }
        if !o.query.is_empty() {
            url.query_pairs_mut().extend_pairs(&o.query);
        }
        let agent = agent(&AgentKey {
            proxy: o.proxy.clone(),
            ip_family: o.ip_family,
            ca: o.ca.clone(),
            cert: o.cert.clone().zip(o.key.clone()),
        })?;
        let mut body = body_of(o)?;
        let mut headers: Vec<(HeaderName, HeaderValue)> = Vec::new();
        let mut push = |name: &str, value: &str| -> Result<(), String> {
            let n = HeaderName::from_bytes(name.trim().as_bytes())
                .map_err(|_| format!("jina la kichwa si sahihi: {name}"))?;
            let v = HeaderValue::from_str(value)
                .map_err(|_| format!("thamani ya kichwa '{name}' si sahihi"))?;
            headers.push((n, v));
            Ok(())
        };
        for (k, v) in &o.headers {
            push(k, v)?;
        }
        if let Some(user) = &o.user {
            use base64::Engine;
            let pair = format!("{user}:{}", o.password.as_deref().unwrap_or(""));
            let encoded = base64::engine::general_purpose::STANDARD.encode(pair);
            push("authorization", &format!("Basic {encoded}"))?;
        } else if let Some(token) = &o.token {
            push("authorization", &format!("Bearer {token}"))?;
        }
        let explicit_type = o
            .content_type
            .clone()
            .or_else(|| match &body {
                Body::Bytes(_, t) => t.map(str::to_string),
                Body::File(_) => Some("application/octet-stream".into()),
                _ => None,
            })
            .filter(|_| {
                !o.headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            });
        if let Some(t) = &explicit_type {
            push("content-type", t)?;
        }
        let max_redirects = o.redirects.unwrap_or(super::MAX_REDIRECTS);
        let timeout = o.timeout.unwrap_or(super::TIMEOUT_SECS);
        let mut redirects = 0;
        let mut attempt = 0;
        let mut response = loop {
            let mut builder = http::Request::builder()
                .method(method.clone())
                .uri(url.as_str());
            for (k, v) in &headers {
                builder = builder.header(k, v);
            }
            if o.cookies {
                let jar = jar().lock().unwrap_or_else(|e| e.into_inner());
                let cookie = jar
                    .get_request_values(&url)
                    .map(|(n, v)| format!("{n}={v}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                if !cookie.is_empty() {
                    builder = builder.header(header::COOKIE, cookie);
                }
            }
            let result = run(&agent, builder, &body, o, timeout);
            let result = match result {
                Ok(r) => r,
                // A connection that failed is retried like a 503.
                Err(_) if attempt < o.retries && idempotent(&method) => {
                    attempt += 1;
                    std::thread::sleep(backoff(attempt, None));
                    continue;
                }
                Err(e) => return Err(e),
            };
            if o.cookies {
                let set = result
                    .headers()
                    .get_all(header::SET_COOKIE)
                    .iter()
                    .filter_map(|v| v.to_str().ok())
                    .filter_map(|s| cookie_store::RawCookie::parse(s.to_string()).ok())
                    .collect::<Vec<_>>();
                jar()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .store_response_cookies(set.into_iter(), &url);
            }
            let status = result.status().as_u16();
            if matches!(status, 429 | 502 | 503 | 504) && attempt < o.retries && idempotent(&method)
            {
                attempt += 1;
                std::thread::sleep(backoff(attempt, retry_after(&result)));
                continue;
            }
            let location = result
                .headers()
                .get(header::LOCATION)
                .and_then(|l| l.to_str().ok())
                .map(str::to_string);
            match (status, location) {
                (301 | 302 | 303 | 307 | 308, Some(location)) if redirects < max_redirects => {
                    let next = url
                        .join(&location)
                        .map_err(|e| format!("kuelekezwa kwenda '{location}': {e}"))?;
                    if url.scheme() == "https" && next.scheme() != "https" {
                        return Err(format!(
                            "kuelekezwa kutoka HTTPS kwenda {} kumekataliwa",
                            next.scheme()
                        ));
                    }
                    if !matches!(next.scheme(), "http" | "https") {
                        return Err(format!("kuelekezwa kwenda '{next}' kumekataliwa"));
                    }
                    if next.origin() != url.origin() {
                        headers.retain(|(k, _)| {
                            *k != header::AUTHORIZATION
                                && *k != header::PROXY_AUTHORIZATION
                                && *k != header::COOKIE
                        });
                    }
                    // 303, and 301/302 after a POST, continue as a GET without the body.
                    if status == 303 || (matches!(status, 301 | 302) && method == Method::POST) {
                        if method != Method::HEAD {
                            method = Method::GET;
                        }
                        body = Body::Empty;
                        headers.retain(|(k, _)| *k != header::CONTENT_TYPE);
                    }
                    redirects += 1;
                    url = next;
                }
                _ => break result,
            }
        };
        let status = response.status();
        let version = format!("{:?}", response.version());
        let reason = status.canonical_reason().unwrap_or("").to_string();
        let resp_headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                )
            })
            .collect();
        let charset = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|t| {
                t.split(';')
                    .filter_map(|p| p.trim().split_once('='))
                    .find(|(k, _)| k.trim().eq_ignore_ascii_case("charset"))
                    .map(|(_, v)| v.trim().trim_matches('"').to_string())
            });
        let limit = o.limit.unwrap_or(super::BODY_LIMIT);
        let reader = response.body_mut().with_config().limit(limit).reader();
        let body = if let Some(path) = &o.save_to {
            save(reader, path)?;
            String::new()
        } else {
            let mut bytes = Vec::new();
            let mut reader = reader;
            reader.read_to_end(&mut bytes).map_err(read_error)?;
            if o.base64_response {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(bytes)
            } else {
                decode(bytes, charset.as_deref())
            }
        };
        Ok(Response {
            status: status.as_u16(),
            reason,
            version,
            headers: resp_headers,
            body,
            url: url.to_string(),
            seconds: started.elapsed().as_secs_f64(),
        })
    }

    fn run(
        agent: &ureq::Agent,
        builder: http::request::Builder,
        body: &Body,
        o: &Options,
        timeout: f64,
    ) -> Result<http::Response<ureq::Body>, String> {
        let secs = |s: f64| (s > 0.0).then(|| Duration::from_secs_f64(s));
        macro_rules! send {
            ($builder:expr, $body:expr) => {{
                let request = $builder.body($body).map_err(|e| e.to_string())?;
                let request = agent
                    .configure_request(request)
                    .timeout_global(secs(timeout))
                    .timeout_connect(o.connect_timeout.and_then(secs))
                    .build();
                agent.run(request).map_err(describe)
            }};
        }
        match body {
            Body::Empty => send!(builder, ()),
            Body::Bytes(bytes, _) => send!(builder, bytes.as_slice()),
            Body::File(path) => {
                let file = std::fs::File::open(path).map_err(|e| format!("faili {path}: {e}"))?;
                send!(builder, file)
            }
            Body::Multipart => {
                let mut form = ureq::unversioned::multipart::Form::new();
                let builder = if builder
                    .headers_ref()
                    .is_some_and(|h| h.contains_key(header::CONTENT_TYPE))
                {
                    builder
                } else {
                    let kind = format!("multipart/form-data; boundary={}", form.boundary());
                    builder.header(header::CONTENT_TYPE, kind)
                };
                for (k, v) in &o.multipart_text {
                    form = form.text(k, v);
                }
                for (k, path) in &o.multipart_files {
                    form = form
                        .file(k, path)
                        .map_err(|e| format!("fomu_faili {path}: {e}"))?;
                }
                send!(builder, form)
            }
        }
    }

    /// Stream the body to `path`, through a temporary file renamed into place when complete,
    /// so a failed download never leaves a partial file under the requested name.
    /// `reader` stops with an error at the size limit.
    fn save(mut reader: impl Read, path: &str) -> Result<(), String> {
        let tmp = format!("{path}.sehemu");
        let result = (|| {
            let mut file = std::fs::File::create(&tmp)?;
            std::io::copy(&mut reader, &mut file)?;
            file.sync_all()?;
            std::fs::rename(&tmp, path)
        })();
        result.map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("hifadhi {path}: {}", read_error(e))
        })
    }

    /// A body read's error; `ureq`'s own (the size limit, a timeout) come wrapped in `io::Error`.
    fn read_error(e: std::io::Error) -> String {
        match e.into_inner().map(|inner| inner.downcast::<ureq::Error>()) {
            Some(Ok(e)) => describe(*e),
            Some(Err(inner)) => format!("kusoma jibu: {inner}"),
            None => "kusoma jibu kumeshindwa".into(),
        }
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

    fn idempotent(m: &Method) -> bool {
        matches!(
            *m,
            Method::GET
                | Method::HEAD
                | Method::PUT
                | Method::DELETE
                | Method::OPTIONS
                | Method::TRACE
        )
    }

    /// 0.5 s, 1 s, 2 s, … (or the server's `Retry-After`), never more than 30 s.
    fn backoff(attempt: u32, server: Option<f64>) -> Duration {
        let wait = server.unwrap_or(0.25 * 2f64.powi(attempt as i32));
        Duration::from_secs_f64(wait.clamp(0.0, super::MAX_RETRY_WAIT_SECS))
    }

    fn describe(e: ureq::Error) -> String {
        match e {
            ureq::Error::Timeout(_) => "muda umekwisha".into(),
            ureq::Error::BodyExceedsLimit(n) => format!("jibu limezidi kikomo cha baiti {n}"),
            other => other.to_string(),
        }
    }
}
