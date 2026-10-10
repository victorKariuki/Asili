//! Compression: choosing an encoding from `Accept-Encoding`, and gzip / brotli.

use std::io::Write;

/// The encoding to answer with, from a request's `Accept-Encoding`: brotli, then gzip; one
/// with `q=0` is refused.
pub fn choose(accept_encoding: &str) -> Option<&'static str> {
    let accepts = |name: &str| {
        accept_encoding.split(',').any(|item| {
            let mut parts = item.split(';');
            let token = parts.next().unwrap_or("").trim();
            let q = parts
                .find_map(|p| p.trim().strip_prefix("q="))
                .and_then(|q| q.trim().parse::<f32>().ok())
                .unwrap_or(1.0);
            (token.eq_ignore_ascii_case(name) || token == "*") && q > 0.0
        })
    };
    ["br", "gzip"].into_iter().find(|e| accepts(e))
}

/// Whether a body of this type is worth compressing (text; not images, archives, video).
pub fn compressible(content_type: Option<&str>) -> bool {
    let Some(t) = content_type else {
        return true;
    };
    let t = t.to_ascii_lowercase();
    t.starts_with("text/")
        || [
            "json",
            "javascript",
            "xml",
            "svg",
            "wasm",
            "x-www-form-urlencoded",
        ]
        .iter()
        .any(|k| t.contains(k))
}

/// `data` encoded with `encoding` (`"gzip"` or `"br"`).
pub fn compress(encoding: &str, data: &[u8]) -> Option<Vec<u8>> {
    match encoding {
        "gzip" => {
            let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
            e.write_all(data).ok()?;
            e.finish().ok()
        }
        "br" => {
            let mut out = Vec::new();
            {
                let mut w = brotli::CompressorWriter::new(&mut out, 4096, 5, 22);
                w.write_all(data).ok()?;
            }
            Some(out)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_by_preference_and_q() {
        assert_eq!(choose("gzip, deflate, br"), Some("br"));
        assert_eq!(choose("gzip"), Some("gzip"));
        assert_eq!(choose("br;q=0, gzip;q=0.5"), Some("gzip"));
        assert_eq!(choose("identity"), None);
        assert!(compressible(Some("application/json; charset=utf-8")));
        assert!(!compressible(Some("image/png")));
        let data = b"habari ".repeat(200);
        let gz = compress("gzip", &data).unwrap();
        assert!(gz.len() < data.len());
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut flate2::read::GzDecoder::new(&gz[..]), &mut out).unwrap();
        assert_eq!(out, data);
        assert!(compress("br", &data).unwrap().len() < data.len());
    }
}
