//! HTTP client (`mfumo`): `http_pata` (GET), `http_tuma` (POST) and `http_ombi` (any method,
//! headers, and the whole response as a `JibuHttp`). One agent serves the whole program, so
//! connections to a host are kept alive and reused across requests; HTTPS uses rustls with the
//! Mozilla root certificates, and gzip responses are decoded.

use std::collections::HashMap;

use super::BuiltinFn;
use crate::value::{self, EvalError, MapKey, Value};

/// Largest response body read (bytes).
const BODY_LIMIT: u64 = 256 * 1024 * 1024;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "http_pata".to_string(),
        Box::new(|args: &[Value]| {
            let url = super::arg_str(args, 0);
            Ok(body_or_error(&url, request("GET", &url, &[], None)))
        }),
    );
    m.insert(
        "http_tuma".to_string(),
        Box::new(|args: &[Value]| {
            let url = super::arg_str(args, 0);
            let body = super::arg_str(args, 1);
            let kind = super::arg_str(args, 2);
            let headers = [("Content-Type".to_string(), kind)];
            Ok(body_or_error(
                &url,
                request("POST", &url, &headers, Some(body)),
            ))
        }),
    );
    m.insert(
        "http_ombi".to_string(),
        Box::new(|args: &[Value]| {
            let method = super::arg_str(args, 0).to_ascii_uppercase();
            let url = super::arg_str(args, 1);
            let headers: Vec<(String, String)> = match args.get(2) {
                Some(Value::Kamusi(m)) => m
                    .iter()
                    .filter_map(|(k, v)| match (k, value::as_string(v)) {
                        (MapKey::Neno(k), Some(v)) => Some((k.to_string(), v)),
                        _ => None,
                    })
                    .collect(),
                _ => {
                    return Err(EvalError::TypeErr(
                        "http_ombi inahitaji njia, anwani, Kamusi ya vichwa na mwili".into(),
                    ))
                }
            };
            let body = super::arg_str(args, 3);
            let body =
                (!body.is_empty() || !matches!(method.as_str(), "GET" | "HEAD")).then_some(body);
            Ok(match request(&method, &url, &headers, body) {
                Ok(response) => Value::sawa(response.into_value()),
                Err(e) => Value::kosa(format!("{url}: {e}")),
            })
        }),
    );
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Response {
    /// A `JibuHttp { hali, vichwa, mwili }`, the shape the HTTP server takes back.
    fn into_value(self) -> Value {
        let mut headers =
            crate::value::Kamusi::with_capacity_and_hasher(self.headers.len(), Default::default());
        for (k, v) in self.headers {
            headers.insert(MapKey::Neno(k.into()), Value::neno(v));
        }
        Value::Struct(
            "JibuHttp".into(),
            vec![
                ("hali".into(), Value::Namba(self.status as f64)),
                ("vichwa".into(), Value::Kamusi(std::rc::Rc::new(headers))),
                ("mwili".into(), Value::neno(self.body)),
            ]
            .into(),
        )
    }
}

/// `Tokeo` of a response's body: the body for a 2xx status, else an error with the status.
fn body_or_error(url: &str, response: Result<Response, String>) -> Value {
    match response {
        Ok(r) if (200..300).contains(&r.status) => Value::sawa(Value::neno(r.body)),
        Ok(r) => {
            let mut snippet: String = r.body.chars().take(200).collect();
            if snippet.len() < r.body.len() {
                snippet.push('…');
            }
            Value::kosa(format!("{url}: HTTP {}: {}", r.status, snippet.trim_end()))
        }
        Err(e) => Value::kosa(format!("{url}: {e}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn agent() -> &'static ureq::Agent {
    static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
    AGENT.get_or_init(|| {
        // The crypto provider the TLS server uses (aws-lc-rs), so the program holds one.
        let crypto = std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        ureq::Agent::config_builder()
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .provider(ureq::tls::TlsProvider::Rustls)
                    .unversioned_rustls_crypto_provider(crypto)
                    .build(),
            )
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(60)))
            .build()
            .into()
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn request(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<String>,
) -> Result<Response, String> {
    let mut builder = ureq::http::Request::builder().method(method).uri(url);
    for (k, v) in headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    let result = match body {
        Some(body) => builder
            .body(body)
            .map_err(|e| e.to_string())
            .and_then(|req| agent().run(req).map_err(|e| e.to_string())),
        None => builder
            .body(())
            .map_err(|e| e.to_string())
            .and_then(|req| agent().run(req).map_err(|e| e.to_string())),
    };
    let mut response = result?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect();
    let body = response
        .body_mut()
        .with_config()
        .limit(BODY_LIMIT)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

#[cfg(target_arch = "wasm32")]
fn request(
    _method: &str,
    _url: &str,
    _headers: &[(String, String)],
    _body: Option<String>,
) -> Result<Response, String> {
    Err("HTTP haipatikani kwenye kivinjari".into())
}
