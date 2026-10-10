//! HTTP messages as Asili values — the one place `OmbiHttp`, `JibuHttp` and `ChaguoHttp` are
//! built from and read back into Rust, for the server (`http.rs`) and the client
//! (`http_mteja.rs`) alike. Field order follows the declarations in
//! `core/parser/src/builtins.rs` (`BUILTIN_MODULES`), so field reads by slot hit first time.
// Only the network builtins use these, and the browser has none.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

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
/// `OmbiHttp` for a request the server received.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_to_value(r: asili_mtandao::http::seva::Ombi) -> Value {
    // A repeated parameter keeps its last value.
    let mut query =
        crate::value::Kamusi::with_capacity_and_hasher(r.query.len(), Default::default());
    for (k, v) in r.query {
        query.insert(MapKey::Neno(k.into()), Value::neno(v));
    }
    let query = Value::Kamusi(Rc::new(query));
    Value::Struct(
        "OmbiHttp".into(),
        vec![
            ("njia".into(), Value::neno(r.method)),
            ("anwani".into(), Value::neno(r.target)),
            ("vichwa".into(), headers_to_kamusi(&r.headers)),
            (
                "mwili".into(),
                Value::neno(String::from_utf8_lossy(&r.body).into_owned()),
            ),
            ("sehemu".into(), Value::neno(r.path)),
            ("hoja".into(), query),
            ("mwili_baiti".into(), Value::Baiti(r.body.into())),
            ("mteja".into(), Value::neno(r.peer)),
            ("toleo".into(), Value::neno(r.version.to_string())),
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

/// A handler's `JibuHttp` read back for sending: status, reason, headers (`vichwa`, then
/// `vichwa_vyote`, which can repeat a name) and body — `mwili_njia`'s chunks (a channel) when
/// given, else `mwili_baiti`, else `mwili`. Any struct with `hali` works — the struct's name is
/// not checked, as with the JSON codec. `None` without a numeric `hali`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn value_to_response(v: &Value) -> Option<asili_mtandao::http::seva::Jibu> {
    use asili_mtandao::http::seva::{Jibu, JibuMwili};
    let status = value::as_f64(field(v, "hali"))?;
    let mut headers = kamusi_to_pairs(field(v, "vichwa")).unwrap_or_default();
    headers.extend(list_to_pairs(field(v, "vichwa_vyote")));
    let body = match (field(v, "mwili_njia"), field(v, "mwili_baiti")) {
        (Value::NjiaRx(rx) | Value::NjiaRxBounded(rx), _) => {
            JibuMwili::Stream(Box::pin(Chunks(rx.clone().into_stream())))
        }
        (_, Value::Baiti(b)) => JibuMwili::Full(b.to_vec()),
        _ => JibuMwili::Full(
            value::as_string(field(v, "mwili"))
                .unwrap_or_default()
                .into_bytes(),
        ),
    };
    let reason = value::as_string(field(v, "sababu")).filter(|r| !r.is_empty());
    Some(Jibu {
        status: status as u16,
        reason,
        headers,
        body,
        websocket: None,
    })
}

/// A channel's values as body chunks: text and bytes as they are, anything else as it prints.
#[cfg(not(target_arch = "wasm32"))]
struct Chunks(flume::r#async::RecvStream<'static, value::SendValue>);

#[cfg(not(target_arch = "wasm32"))]
impl futures_core::Stream for Chunks {
    type Item = Vec<u8>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Vec<u8>>> {
        std::pin::Pin::new(&mut self.0).poll_next(cx).map(|item| {
            item.map(|v| match v.into_value() {
                Value::Baiti(b) => b.to_vec(),
                other => value::to_display_string(&other)
                    .unwrap_or_else(|| format!("{other:?}"))
                    .into_bytes(),
            })
        })
    }
}

/// One server-sent event (`text/event-stream`): `id:` and `event:` lines when given, a `data:`
/// line per line of `data`, then a blank line.
pub(crate) fn sse_event(data: &str, event: Option<&str>, id: Option<&str>) -> String {
    let mut out = String::new();
    if let Some(id) = id {
        out.push_str(&format!("id: {}\n", id.replace(['\r', '\n'], "")));
    }
    if let Some(e) = event {
        out.push_str(&format!("event: {}\n", e.replace(['\r', '\n'], "")));
    }
    for line in data.split('\n') {
        out.push_str(&format!("data: {}\n", line.trim_end_matches('\r')));
    }
    out.push('\n');
    out
}
