//! HTTP on hyper: the server ([`seva`]), and what it shares with the client — compression
//! ([`bana`]). Messages are plain data (names, values, bytes); turning them into a program's
//! values is the caller's.

pub mod bana;
pub mod seva;

/// The protocol name of an HTTP version, as messages show it.
pub fn version_name(v: hyper::Version) -> &'static str {
    match v {
        hyper::Version::HTTP_09 => "HTTP/0.9",
        hyper::Version::HTTP_10 => "HTTP/1.0",
        hyper::Version::HTTP_2 => "HTTP/2",
        hyper::Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/1.1",
    }
}

/// Header names (lowercase) and values (invalid UTF-8 replaced), in order.
pub fn header_pairs(h: &hyper::HeaderMap) -> Vec<(String, String)> {
    h.iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect()
}
