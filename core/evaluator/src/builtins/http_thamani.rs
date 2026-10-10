//! HTTP messages as Asili values — the one place `OmbiHttp`, `JibuHttp` and `ChaguoHttp` are
//! built from and read back into Rust, for the server (`http.rs`) and the client
//! (`http_mteja.rs`) alike. Field order follows the declarations in
//! `core/parser/src/builtins.rs` (`BUILTIN_MODULES`), so field reads by slot hit first time.

use crate::value::{self, MapKey, Value};
use std::rc::Rc;

/// `Chaguo::Kuna(x)` (either runtime shape) is `x`; `Chaguo::Hamna` is `Hamna`.
pub(crate) fn unwrap_some(v: &Value) -> &Value {
    match v {
        Value::Chaguo(Some(inner)) => inner,
        Value::Chaguo(None) => &Value::Hamna,
        Value::Enum(e, variant, data) if &**e == "Chaguo" => match (&**variant, data) {
            ("Kuna", Some(inner)) => inner,
            _ => &Value::Hamna,
        },
        other => other,
    }
}

/// Field `name` of a struct value (`Hamna` when absent), with `Chaguo::Kuna` unwrapped.
pub(crate) fn field<'a>(v: &'a Value, name: &str) -> &'a Value {
    match v {
        Value::Struct(_, fields) => fields
            .iter()
            .find(|(n, _)| &**n == name)
            .map_or(&Value::Hamna, |(_, v)| unwrap_some(v)),
        _ => &Value::Hamna,
    }
}

/// A `Kamusi<Neno, Neno>` as ordered pairs (sorted, so a message is the same every run).
pub(crate) fn kamusi_to_pairs(v: &Value) -> Option<Vec<(String, String)>> {
    let Value::Kamusi(m) = v else {
        return None;
    };
    let mut pairs = m
        .iter()
        .map(|(k, v)| match k {
            MapKey::Neno(k) => Some((k.to_string(), value::as_string(v)?)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    pairs.sort();
    Some(pairs)
}

/// Headers as `vichwa`: a `Kamusi` of lowercase names, a repeated header joined with `", "`
/// (RFC 9110 §5.3).
pub(crate) fn headers_to_kamusi(headers: &[(String, String)]) -> Value {
    let mut map = crate::value::Kamusi::with_capacity_and_hasher(headers.len(), Default::default());
    for (k, v) in headers {
        let key = MapKey::Neno(k.to_ascii_lowercase().into());
        let joined = match map.get(&key).and_then(value::as_string) {
            Some(existing) => format!("{existing}, {v}"),
            None => v.clone(),
        };
        map.insert(key, Value::neno(joined));
    }
    Value::Kamusi(Rc::new(map))
}

/// Headers as `vichwa_vyote`: every header in order, as `Jozi`, repeats kept apart.
pub(crate) fn headers_to_list(headers: &[(String, String)]) -> Value {
    Value::Orodha(Rc::new(
        headers
            .iter()
            .map(|(k, v)| {
                Value::Jozi(
                    Box::new(Value::neno(k.to_ascii_lowercase())),
                    Box::new(Value::neno(v.clone())),
                )
            })
            .collect(),
    ))
}

/// `vichwa_vyote` read back: `Orodha<Jozi<Neno, Neno>>` as pairs.
fn list_to_pairs(v: &Value) -> Vec<(String, String)> {
    let Value::Orodha(items) = v else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| match item {
            Value::Jozi(k, v) => Some((value::as_string(k)?, value::as_string(v)?)),
            _ => None,
        })
        .collect()
}

/// A request as the server hands it to a handler.
pub(crate) struct Request {
    pub method: String,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// `OmbiHttp { njia, anwani, vichwa, mwili }`.
pub(crate) fn request_to_value(r: Request) -> Value {
    Value::Struct(
        "OmbiHttp".into(),
        vec![
            ("njia".into(), Value::neno(r.method)),
            ("anwani".into(), Value::neno(r.target)),
            ("vichwa".into(), headers_to_kamusi(&r.headers)),
            ("mwili".into(), Value::neno(r.body)),
        ]
        .into(),
    )
}

/// A response: what the client received, or what a server handler returned.
pub(crate) struct Response {
    pub status: u16,
    pub reason: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub url: String,
    pub seconds: f64,
}

/// `JibuHttp`, every field filled (the client's view).
pub(crate) fn response_to_value(r: Response) -> Value {
    Value::Struct(
        "JibuHttp".into(),
        vec![
            ("hali".into(), Value::Namba(r.status as f64)),
            ("vichwa".into(), headers_to_kamusi(&r.headers)),
            ("mwili".into(), Value::neno(r.body)),
            ("sababu".into(), Value::neno(r.reason)),
            ("anwani".into(), Value::neno(r.url)),
            ("toleo".into(), Value::neno(r.version)),
            ("vichwa_vyote".into(), headers_to_list(&r.headers)),
            ("muda".into(), Value::Namba(r.seconds)),
        ]
        .into(),
    )
}

/// A handler's `JibuHttp` read back for sending: status, headers (`vichwa`, then
/// `vichwa_vyote`, which can repeat a name) and body. Any struct with `hali` works — the
/// struct's name is not checked, as with the JSON codec. `None` without a numeric `hali`.
pub(crate) fn value_to_response(v: &Value) -> Option<(u16, Vec<(String, String)>, String)> {
    let status = value::as_f64(field(v, "hali"))?;
    let body = value::as_string(field(v, "mwili")).unwrap_or_default();
    let mut headers = kamusi_to_pairs(field(v, "vichwa")).unwrap_or_default();
    headers.extend(list_to_pairs(field(v, "vichwa_vyote")));
    Some((status as u16, headers, body))
}

/// The reason phrase for `status` (RFC 9110 and registered extensions; `""` when unknown).
pub(crate) fn reason_phrase(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Content Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Content",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _ => "",
    }
}
