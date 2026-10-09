//! Built-in functions for HTTP requests, regular expressions, encodings and hashes: each snippet
//! against a hand-checked expectation, run as native code. HTTP goes to a small server on this
//! machine.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

fn run(source: &str) -> String {
    let tokens = tokenize(source).expect("tokenize");
    let module = parse_tokens(&tokens).expect("parse");
    match run_function(&module, "t", vec![]) {
        Ok(Value::Neno(s)) => s.to_string(),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("kosa: {e}"),
    }
}

fn check(body: &str, expected: &str) {
    let source = format!(
        "leta mfumo\nleta ruwaza\nleta usimbaji\nleta hisabati\nkazi t() -> Neno {{\n{body}\n}}\n"
    );
    assert_eq!(run(&source), expected, "\n{source}");
}

/// A server answering every request with its method, path, `X-Jina` header and body, as
/// `200 OK` — or `404` for `/hakuna`. Returns its address.
fn echo_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            let mut length = 0usize;
            let mut name = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                let (k, v) = line.split_once(':').unwrap_or((&line, ""));
                match k.to_ascii_lowercase().as_str() {
                    "content-length" => length = v.trim().parse().unwrap_or(0),
                    "x-jina" => name = v.trim().to_string(),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let mut parts = request_line.split_whitespace();
            let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            let status = if path == "/hakuna" {
                "404 Not Found"
            } else {
                "200 OK"
            };
            let reply = format!("{method} {path} {name} {}", String::from_utf8_lossy(&body));
            let _ = write!(
                reader.get_mut(),
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nX-Seva: asili\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    format!("http://{addr}")
}

#[test]
fn http_requests() {
    let base = echo_server();
    check(
        &format!(r#"rejesha http_pata("{base}/habari").angu("")"#),
        "GET /habari  ",
    );
    check(
        &format!(r#"rejesha http_pata("{base}/hakuna").kosa()"#),
        &format!("{base}/hakuna: HTTP 404: GET /hakuna"),
    );
    check(
        &format!(
            r#"rejesha http_tuma("{base}/tuma", "{{\"a\": 1}}", "application/json").angu("")"#
        ),
        r#"POST /tuma  {"a": 1}"#,
    );
    check(
        &format!(
            r#"weka vichwa = kamusi_tupu()
vichwa.ingiza("X-Jina", "Amara")
weka j = http_ombi("put", "{base}/x", vichwa, "mwili").angu(kamusi_tupu())
rejesha (j.hali kama Neno) + " " + j.vichwa.pata("x-seva").angu("") + " " + j.mwili"#
        ),
        "200 asili PUT /x Amara mwili",
    );
    // A status other than 2xx is still a response to `http_ombi`.
    check(
        &format!(
            r#"rejesha (http_ombi("GET", "{base}/hakuna", kamusi_tupu(), "").angu(kamusi_tupu()).hali kama Neno)"#
        ),
        "404",
    );
    check(
        r#"rejesha http_pata("http://127.0.0.1:1/").ni_kosa() kama Neno"#,
        "kweli",
    );
}

/// HTTPS to a server whose certificate no trusted authority signed: the handshake runs (with
/// the same crypto provider as the TLS server, in the same process) and the certificate is
/// refused — an error, not a crash.
#[test]
fn https_checks_the_certificate() {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let key = rustls::pki_types::PrivateKeyDer::try_from(key_pair.serialize_der()).unwrap();
    let config = std::sync::Arc::new(
        rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()], key)
        .expect("server config"),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut tcp, _)) = listener.accept() {
            let mut conn = rustls::ServerConnection::new(config).unwrap();
            let _ = conn.complete_io(&mut tcp);
        }
    });
    let source = format!(
        "leta mfumo\nkazi t() -> Neno {{\nrejesha http_pata(\"https://localhost:{port}/\").kosa()\n}}\n"
    );
    let error = run(&source);
    assert!(
        error.contains("https://localhost:") && error.to_lowercase().contains("certificate"),
        "{error}"
    );
}

#[test]
fn regular_expressions() {
    check(
        r#"rejesha ruwaza_inalingana("^\\d{3}-\\d{4}$", "555-1234").angu(si_kweli) kama Neno"#,
        "kweli",
    );
    check(
        r#"rejesha ruwaza_tafuta("[a-z]+", "123 habari 456").angu(Hamna).angu("")"#,
        "habari",
    );
    check(
        r#"rejesha ruwaza_zote("\\d+", "a1 b22 c333").angu([]).jiunge(",")"#,
        "1,22,333",
    );
    check(
        r#"rejesha ruwaza_vikundi("(\\w+)@(\\w+)\\.com(x)?", "barua: juma@mfano.com").angu(Hamna).angu([]).jiunge("|")"#,
        "juma@mfano.com|juma|mfano|",
    );
    check(
        r#"rejesha ruwaza_vikundi("x", "abc").angu(Hamna).ni_tupu() kama Neno"#,
        "kweli",
    );
    check(
        r#"rejesha ruwaza_badilisha("(\\w+) (\\w+)", "habari dunia", "$2 $1").angu("")"#,
        "dunia habari",
    );
    check(
        r#"rejesha ruwaza_gawanya("\\s*,\\s*", "a , b,c").angu([]).jiunge("|")"#,
        "a|b|c",
    );
    // Positions and classes are Unicode-aware.
    check(
        r#"rejesha ruwaza_zote("\\w+", "café ñandú").angu([]).jiunge("|")"#,
        "café|ñandú",
    );
    check(
        r#"rejesha ruwaza_inalingana("(", "x").kosa()"#,
        "ruwaza '(' si sahihi: regex parse error:\n    (\n    ^\nerror: unclosed group",
    );
}

#[test]
fn encodings_and_hashes() {
    check(
        r#"rejesha base64_simba("habari dunia")"#,
        "aGFiYXJpIGR1bmlh",
    );
    check(
        r#"rejesha base64_fumbua(base64_simba("café ☕")).angu("")"#,
        "café ☕",
    );
    check(
        r#"rejesha base64_fumbua("@@@").ni_kosa() kama Neno"#,
        "kweli",
    );
    check(r#"rejesha hex_simba("Aé")"#, "41c3a9");
    check(
        r#"rejesha hashi_sha256("")"#,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
    check(
        r#"rejesha hashi_sha512("abc").kata(0, 16)"#,
        "ddaf35a193617aba",
    );
    // RFC 4231 test case 2.
    check(
        r#"rejesha hmac_sha256("Jefe", "what do ya want for nothing?")"#,
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
    );
    check(
        r#"weka id = kitambulisho()
rejesha (id.urefu() kama Neno) + " " + id.kata(14, 15) + " " + (ruwaza_inalingana("^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$", id).angu(si_kweli) kama Neno)"#,
        "36 4 kweli",
    );
    check(
        r#"nasibu_mbegu(9)
weka a = kitambulisho()
nasibu_mbegu(9)
rejesha (a == kitambulisho()) kama Neno"#,
        "kweli",
    );
}
