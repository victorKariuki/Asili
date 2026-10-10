//! Usimbaji (encodings and hashes): base64 and hexadecimal, SHA-256 and SHA-512, HMAC-SHA256,
//! and random UUIDs (version 4).

use std::collections::HashMap;

use base64::Engine as _;
use sha2::Digest as _;

use super::BuiltinFn;
use crate::value::Value;

/// `bytes` as lowercase hexadecimal.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(2 * bytes.len());
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
    out
}

/// HMAC with SHA-256 (RFC 2104).
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&sha2::Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let pad = |byte: u8| k.map(|b| b ^ byte);
    let inner = sha2::Sha256::new()
        .chain_update(pad(0x36))
        .chain_update(message)
        .finalize();
    sha2::Sha256::new()
        .chain_update(pad(0x5c))
        .chain_update(inner)
        .finalize()
        .into()
}

/// A random (version 4) UUID, from this thread's random numbers (so `nasibu_mbegu` makes them
/// repeat too).
fn uuid_v4() -> String {
    let mut b: [u8; 16] = super::hisabati::random_bytes();
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80; // RFC 4122 variant
    let h = hex(&b);
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn text_to_text(m: &mut HashMap<String, BuiltinFn>, name: &str, f: fn(&str) -> String) {
    m.insert(
        name.to_string(),
        Box::new(move |args: &[Value]| Ok(Value::neno(f(&super::arg_str(args, 0))))),
    );
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    text_to_text(m, "base64_simba", |s| {
        base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
    });
    text_to_text(m, "hex_simba", |s| hex(s.as_bytes()));
    text_to_text(m, "hashi_sha256", |s| {
        hex(&sha2::Sha256::digest(s.as_bytes()))
    });
    text_to_text(m, "hashi_sha512", |s| {
        hex(&sha2::Sha512::digest(s.as_bytes()))
    });
    m.insert(
        "base64_fumbua".to_string(),
        Box::new(|args: &[Value]| {
            let text = super::arg_str(args, 0);
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(text.trim())
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    String::from_utf8(bytes).map_err(|_| "si maandishi ya UTF-8".into())
                });
            Ok(match decoded {
                Ok(s) => Value::sawa(Value::neno(s)),
                Err(e) => Value::kosa(format!("base64_fumbua: {e}")),
            })
        }),
    );
    m.insert(
        "base64_fumbua_baiti".to_string(),
        Box::new(|args: &[Value]| {
            let text = super::arg_str(args, 0);
            Ok(
                match base64::engine::general_purpose::STANDARD.decode(text.trim()) {
                    Ok(b) => Value::sawa(Value::Baiti(b.into())),
                    Err(e) => Value::kosa(format!("base64_fumbua_baiti: {e}")),
                },
            )
        }),
    );
    m.insert(
        "hex_fumbua".to_string(),
        Box::new(|args: &[Value]| {
            let text = super::arg_str(args, 0);
            let text = text.trim();
            let bytes = (text.len() % 2 == 0)
                .then(|| {
                    (0..text.len() / 2)
                        .map(|i| u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok())
                        .collect::<Option<Vec<u8>>>()
                })
                .flatten();
            Ok(match bytes {
                Some(b) => Value::sawa(Value::Baiti(b.into())),
                None => Value::kosa("hex_fumbua: si hex halali"),
            })
        }),
    );
    m.insert(
        "hmac_sha256".to_string(),
        Box::new(|args: &[Value]| {
            let (key, message) = (super::arg_str(args, 0), super::arg_str(args, 1));
            Ok(Value::neno(hex(&hmac_sha256(
                key.as_bytes(),
                message.as_bytes(),
            ))))
        }),
    );
    m.insert(
        "kitambulisho".to_string(),
        Box::new(|_args: &[Value]| Ok(Value::neno(uuid_v4()))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_answers() {
        // FIPS 180-2 / RFC 4231 test vectors.
        assert_eq!(
            hex(&sha2::Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&hmac_sha256(&[0x0b; 20], b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        let long_key = [0xaa; 131];
        assert_eq!(
            hex(&hmac_sha256(
                &long_key,
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            )),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
        let id = uuid_v4();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert!(matches!(&id[19..20], "8" | "9" | "a" | "b"));
    }
}
