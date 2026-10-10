//! Built-in functions for HTTP requests, regular expressions, encodings and hashes: each snippet
//! against a hand-checked expectation, run as native code. HTTP goes to a small server on this
//! machine.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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

/// One request as the test server sees it.
struct Seen {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Seen {
    fn header(&self, name: &str) -> &str {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map_or("", |(_, v)| v.as_str())
    }
}

fn read_request(reader: &mut impl BufRead) -> Option<Seen> {
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next()?.to_string(), parts.next()?.to_string());
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).ok()? == 0 || h == "\r\n" {
            break;
        }
        let (k, v) = h.split_once(':')?;
        headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
    }
    let mut seen = Seen {
        method,
        path,
        headers,
        body: Vec::new(),
    };
    if seen.header("transfer-encoding") == "chunked" {
        loop {
            let mut size = String::new();
            reader.read_line(&mut size).ok()?;
            let n = usize::from_str_radix(size.trim(), 16).ok()?;
            let mut chunk = vec![0; n + 2];
            reader.read_exact(&mut chunk).ok()?;
            if n == 0 {
                break;
            }
            seen.body.extend_from_slice(&chunk[..n]);
        }
    } else {
        let n: usize = seen.header("content-length").parse().unwrap_or(0);
        seen.body = vec![0; n];
        reader.read_exact(&mut seen.body).ok()?;
    }
    Some(seen)
}

/// The test server's answer to `seen`: (status line, extra headers, body).
fn answer(seen: &Seen, other: &str, busy: &AtomicUsize) -> (String, Vec<String>, Vec<u8>) {
    let echo = || {
        format!(
            "{} {}|{}|{}|{}|{}|{}",
            seen.method,
            seen.path,
            seen.header("content-type"),
            seen.header("authorization"),
            seen.header("cookie"),
            seen.header("x-jina"),
            String::from_utf8_lossy(&seen.body)
        )
        .into_bytes()
    };
    let ok = |body: Vec<u8>| ("200 OK".to_string(), vec![], body);
    let path = seen.path.split('?').next().unwrap_or("");
    match path {
        "/hakuna" => ("404 Not Found".into(), vec![], b"haipo".to_vec()),
        "/elekeza" => ("302 Found".into(), vec!["Location: /mwisho".into()], vec![]),
        "/elekeza-nje" => (
            "307 Temporary Redirect".into(),
            vec![format!("Location: {other}/nje")],
            vec![],
        ),
        "/elekeza-http" => (
            "302 Found".into(),
            vec![format!("Location: {other}/mwisho")],
            vec![],
        ),
        "/303" => (
            "303 See Other".into(),
            vec!["Location: /mwisho".into()],
            vec![],
        ),
        "/zunguka" => (
            "302 Found".into(),
            vec!["Location: /zunguka".into()],
            vec![],
        ),
        "/shughuli" => {
            if busy.fetch_add(1, Ordering::SeqCst) < 2 {
                (
                    "503 Service Unavailable".into(),
                    vec!["Retry-After: 0".into()],
                    vec![],
                )
            } else {
                ok(echo())
            }
        }
        "/kuki" => (
            "200 OK".into(),
            vec![
                "Set-Cookie: a=1; Path=/".into(),
                "Set-Cookie: b=2; Path=/".into(),
            ],
            vec![],
        ),
        "/latin1" => (
            "200 OK".into(),
            vec!["Content-Type: text/plain; charset=iso-8859-1".into()],
            vec![b'c', b'a', b'f', 0xE9],
        ),
        "/baiti" => ok(vec![0, 1, 2, 255]),
        "/kubwa" => ok(vec![b'x'; 1000]),
        "/polepole" => {
            std::thread::sleep(std::time::Duration::from_secs(3));
            ok(echo())
        }
        _ => ok(echo()),
    }
}

/// Serve `stream` until it closes (keep-alive), answering as [`answer`] says. As a proxy, it
/// accepts `CONNECT` and then answers the tunnelled requests itself, prefixed `via <target>`.
fn serve(stream: impl Read + Write, other: &str, busy: &AtomicUsize) {
    let mut reader = BufReader::new(stream);
    let mut via = String::new();
    while let Some(seen) = read_request(&mut reader) {
        if seen.method == "CONNECT" {
            via = format!("via {} ", seen.path);
            let out = reader.get_mut();
            if out.write_all(b"HTTP/1.1 200 OK\r\n\r\n").is_err() || out.flush().is_err() {
                return;
            }
            continue;
        }
        let (status, extra, mut body) = answer(&seen, other, busy);
        if !via.is_empty() {
            body.splice(0..0, via.bytes());
        }
        let mut head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\n", body.len());
        for h in extra {
            head.push_str(&h);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let out = reader.get_mut();
        // A HEAD answer has the headers of a GET's, and no body.
        let body = if seen.method == "HEAD" {
            &[][..]
        } else {
            &body[..]
        };
        if out.write_all(head.as_bytes()).is_err()
            || out.write_all(body).is_err()
            || out.flush().is_err()
        {
            return;
        }
    }
}

/// A plain HTTP test server; `other` is the origin its cross-origin redirects point at.
fn server(other: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    let other = other.to_string();
    let busy = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (other, busy) = (other.clone(), busy.clone());
            std::thread::spawn(move || serve(stream, &other, &busy));
        }
    });
    format!("http://{addr}")
}

/// A TLS test server for "localhost" (certificate signed by a fresh CA, written to `ca.pem` in
/// `dir`); with `client_ca`, clients must present a certificate it signed.
fn tls_server(dir: &std::path::Path, client_ca: Option<&rcgen::Certificate>) -> String {
    let ca_key = rcgen::KeyPair::generate().unwrap();
    let mut ca_params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca = ca_params.self_signed(&ca_key).unwrap();
    std::fs::write(dir.join("ca.pem"), ca.pem()).unwrap();
    let key = rcgen::KeyPair::generate().unwrap();
    let cert = rcgen::CertificateParams::new(vec!["localhost".to_string()])
        .unwrap()
        .signed_by(&key, &ca, &ca_key)
        .unwrap();
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = rustls::ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .unwrap();
    let builder = match client_ca {
        Some(client_ca) => {
            let mut roots = rustls::RootCertStore::empty();
            roots.add(client_ca.der().clone()).unwrap();
            let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
                Arc::new(roots),
                provider,
            )
            .build()
            .unwrap();
            builder.with_client_cert_verifier(verifier)
        }
        None => builder.with_no_client_auth(),
    };
    let key_der = rustls::pki_types::PrivateKeyDer::try_from(key.serialize_der()).unwrap();
    let config = Arc::new(
        builder
            .with_single_cert(vec![cert.der().clone()], key_der)
            .expect("server config"),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let plain = server("http://127.0.0.1:1");
    std::thread::spawn(move || {
        let busy = Arc::new(AtomicUsize::new(0));
        for tcp in listener.incoming().flatten() {
            let (config, plain, busy) = (config.clone(), plain.clone(), busy.clone());
            std::thread::spawn(move || {
                let conn = rustls::ServerConnection::new(config).unwrap();
                serve(rustls::StreamOwned::new(conn, tcp), &plain, &busy);
            });
        }
    });
    format!("https://localhost:{port}")
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("asili-http-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run `body`, where `j` is `http_ombi(<call>)`'s response, and return what it returns.
fn ombi(call: &str, body: &str) -> String {
    run(&format!(
        "leta mfumo\nleta usimbaji\nkazi t() -> Neno {{\nweka j = jaribu http_ombi({call})\n{body}\n}}\n"
    ))
}

/// The echo body for `http_ombi(<call>)`.
fn echo(call: &str) -> String {
    ombi(call, "rejesha j.mwili")
}

#[test]
fn http_methods_bodies_and_headers() {
    let base = server("http://127.0.0.1:1");
    assert_eq!(
        echo(&format!(r#""GET", "{base}/habari""#)),
        "GET /habari|||||"
    );
    assert_eq!(
        echo(&format!(
            r#""post", "{base}/tuma", ChaguoHttp {{ mwili: "habari", vichwa: {{"X-Jina": "Amara"}} }}"#
        )),
        "POST /tuma|text/plain; charset=utf-8|||Amara|habari"
    );
    assert_eq!(
        echo(&format!(
            r#""PUT", "{base}/j", ChaguoHttp {{ json: {{"a": 1, "b": 2}} }}"#
        )),
        r#"PUT /j|application/json||||{"a":1,"b":2}"#
    );
    assert_eq!(
        echo(&format!(
            r#""POST", "{base}/f", ChaguoHttp {{ fomu: {{"jina": "Juma Ali", "mji": "Dar & Moshi"}} }}"#
        )),
        "POST /f|application/x-www-form-urlencoded||||jina=Juma+Ali&mji=Dar+%26+Moshi"
    );
    assert_eq!(
        echo(&format!(
            r#""GET", "{base}/q?x=1", ChaguoHttp {{ hoja: {{"neno": "a b", "z": "ü"}} }}"#
        )),
        "GET /q?x=1&neno=a+b&z=%C3%BC|||||"
    );
    // The type given wins over the body's own.
    assert_eq!(
        echo(&format!(
            r#""PATCH", "{base}/p", ChaguoHttp {{ mwili: "<a/>", aina: "application/xml" }}"#
        )),
        "PATCH /p|application/xml||||<a/>"
    );
    assert_eq!(
        echo(&format!(
            r#""POST", "{base}/b", ChaguoHttp {{ mwili_base64: "AAH/" }}"#
        ))
        .split('|')
        .nth(1)
        .unwrap(),
        "application/octet-stream"
    );
    // Any method, standard or not.
    assert_eq!(
        echo(&format!(r#""PROPFIND", "{base}/dav""#)),
        "PROPFIND /dav|||||"
    );
    assert_eq!(
        ombi(
            &format!(r#""HEAD", "{base}/h""#),
            "rejesha (j.hali kama Neno) + j.mwili"
        ),
        "200"
    );
    assert_eq!(
        echo(&format!(r#""DELETE", "{base}/d/7""#)),
        "DELETE /d/7|||||"
    );
    assert!(echo(&format!(
        r#""GET", "{base}", ChaguoHttp {{ mwili: "a", json: 1 }}"#
    ))
    .contains("chagua mwili mmoja tu"));
    assert!(echo(&format!(r#""GE T", "{base}""#)).contains("njia si sahihi"));
    assert!(echo(r#""GET", "ftp://example.com/""#).contains("http:// au https://"));
    assert!(echo(&format!(
        r#""GET", "{base}", ChaguoHttp {{ vichwa: {{"X-Jina": "a\r\nX-Mbaya: 1"}} }}"#
    ))
    .contains("si sahihi"));
}

#[test]
fn http_response_fields() {
    let base = server("http://127.0.0.1:1");
    assert_eq!(
        ombi(
            &format!(r#""GET", "{base}/hakuna""#),
            r#"rejesha (j.hali kama Neno) + " " + j.sababu + " " + j.toleo + " " + j.mwili + " " + j.vichwa.pata("content-length").angu("")"#
        ),
        "404 Not Found HTTP/1.1 haipo 5"
    );
    let error = ombi(
        &format!(r#""GET", "{base}/hakuna", ChaguoHttp {{ kosa_hali: kweli }}"#),
        r#"rejesha "haikufika""#,
    );
    assert!(
        error.contains(&format!("http_ombi GET {base}/hakuna: HTTP 404: haipo")),
        "{error}"
    );
    // Repeated headers: joined in `vichwa`, apart in `vichwa_vyote`.
    assert_eq!(
        ombi(
            &format!(r#""GET", "{base}/kuki""#),
            r#"weka kuki = []
kwa jozi katika j.vichwa_vyote {
    ikiwa jozi.kwanza() == "set-cookie" { kuki.ongeza(jozi.pili()) }
}
rejesha j.vichwa.pata("set-cookie").angu("") + " / " + kuki.jiunge(" + ")"#
        ),
        "a=1; Path=/, b=2; Path=/ / a=1; Path=/ + b=2; Path=/"
    );
    assert_eq!(
        ombi(&format!(r#""GET", "{base}/latin1""#), "rejesha j.mwili"),
        "café"
    );
    assert_eq!(
        ombi(
            &format!(r#""GET", "{base}/baiti", ChaguoHttp {{ jibu_base64: kweli }}"#),
            "rejesha j.mwili"
        ),
        "AAEC/w=="
    );
    assert!(ombi(
        &format!(r#""GET", "{base}/kubwa", ChaguoHttp {{ kikomo: 100 }}"#),
        "rejesha j.mwili"
    )
    .contains("kikomo"));
    assert!(ombi(
        &format!(r#""GET", "{base}/polepole", ChaguoHttp {{ muda: 0.5 }}"#),
        "rejesha j.mwili"
    )
    .contains("muda umekwisha"));
    assert!(ombi(r#""GET", "http://127.0.0.1:1/""#, "rejesha j.mwili").starts_with("kosa: "));
}

#[test]
fn http_redirects_retries_and_cookies() {
    let other = server("http://127.0.0.1:1");
    let base = server(&other);
    assert_eq!(
        ombi(
            &format!(r#""GET", "{base}/elekeza""#),
            r#"rejesha j.mwili + " " + j.anwani"#
        ),
        format!("GET /mwisho||||| {base}/mwisho")
    );
    // 303 turns a POST into a GET without its body.
    assert_eq!(
        echo(&format!(
            r#""POST", "{base}/303", ChaguoHttp {{ mwili: "x" }}"#
        )),
        "GET /mwisho|||||"
    );
    // Leaving the origin drops the credentials; 307 keeps method and body.
    assert_eq!(
        echo(&format!(
            r#""PUT", "{base}/elekeza-nje", ChaguoHttp {{ tokeni: "siri", mwili: "x" }}"#
        )),
        "PUT /nje|text/plain; charset=utf-8||||x"
    );
    assert_eq!(
        echo(&format!(
            r#""GET", "{base}/habari", ChaguoHttp {{ mtumiaji: "juma", nenosiri: "p@ss" }}"#
        )),
        "GET /habari||Basic anVtYTpwQHNz|||"
    );
    assert_eq!(
        ombi(
            &format!(r#""GET", "{base}/elekeza", ChaguoHttp {{ elekezo: 0 }}"#),
            r#"rejesha (j.hali kama Neno) + " " + j.vichwa.pata("location").angu("")"#
        ),
        "302 /mwisho"
    );
    assert!(ombi(
        &format!(r#""GET", "{base}/zunguka""#),
        "rejesha (j.hali kama Neno)"
    )
    .starts_with("302"));
    // Two 503s, then the answer.
    assert_eq!(
        ombi(
            &format!(r#""GET", "{base}/shughuli", ChaguoHttp {{ jaribu_tena: 3 }}"#),
            "rejesha (j.hali kama Neno)"
        ),
        "200"
    );
    // Cookies only when asked, then sent back to the same server.
    assert_eq!(
        ombi(
            &format!(r#""GET", "{base}/kuki", ChaguoHttp {{ vidakuzi: kweli }}"#),
            &format!(
                r#"weka bila = jaribu http_ombi("GET", "{base}/a")
weka na = jaribu http_ombi("GET", "{base}/a", ChaguoHttp {{ vidakuzi: kweli }})
weka nje = jaribu http_ombi("GET", "{}/a", ChaguoHttp {{ vidakuzi: kweli }})
rejesha bila.mwili + " " + na.mwili + " " + nje.mwili"#,
                // Cookies belong to a host, whatever its port: another host gets none.
                other.replace("127.0.0.1", "localhost")
            )
        ),
        "GET /a||||| GET /a|||a=1; b=2|| GET /a|||||"
    );
}

/// A request goes through the proxy given (a tunnel to the target); `wakala: ""` goes direct;
/// `familia_ip` picks the address family.
#[test]
fn http_proxy_and_ip_family() {
    let proxy = server("http://127.0.0.1:1");
    let base = server("http://127.0.0.1:1");
    assert_eq!(
        echo(&format!(
            r#""GET", "http://mfano.invalid/njia?x=1", ChaguoHttp {{ wakala: "{proxy}" }}"#
        )),
        "via mfano.invalid:80 GET /njia?x=1|||||"
    );
    assert_eq!(
        echo(&format!(
            r#""GET", "{base}/moja", ChaguoHttp {{ wakala: "", familia_ip: 4 }}"#
        )),
        "GET /moja|||||"
    );
    assert!(echo(&format!(
        r#""GET", "{base}/", ChaguoHttp {{ familia_ip: 5 }}"#
    ))
    .contains("familia_ip ni 4 au 6"));
}

#[test]
fn http_files_and_multipart() {
    let base = server("http://127.0.0.1:1");
    let dir = temp_dir("faili");
    let upload = dir.join("pakia.txt");
    std::fs::write(&upload, "yaliyomo").unwrap();
    let upload = upload.display();
    assert_eq!(
        echo(&format!(
            r#""PUT", "{base}/pakia", ChaguoHttp {{ faili: "{upload}" }}"#
        )),
        "PUT /pakia|application/octet-stream||||yaliyomo"
    );
    let multipart = echo(&format!(
        r#""POST", "{base}/fomu", ChaguoHttp {{ fomu_sehemu: {{"jina": "Juma"}}, fomu_faili: {{"hati": "{upload}"}} }}"#
    ));
    assert!(
        multipart.starts_with("POST /fomu|multipart/form-data; boundary="),
        "{multipart}"
    );
    assert!(
        multipart.contains("name=\"jina\"\r\n\r\nJuma\r\n"),
        "{multipart}"
    );
    assert!(
        multipart.contains("name=\"hati\"; filename=\"pakia.txt\"")
            && multipart.contains("yaliyomo"),
        "{multipart}"
    );
    let saved = dir.join("hifadhi.bin");
    assert_eq!(
        ombi(
            &format!(
                r#""GET", "{base}/baiti", ChaguoHttp {{ hifadhi: "{}" }}"#,
                saved.display()
            ),
            r#"rejesha (j.hali kama Neno) + j.mwili"#
        ),
        "200"
    );
    assert_eq!(std::fs::read(&saved).unwrap(), vec![0, 1, 2, 255]);
    // A failed download leaves no file behind.
    let failed = dir.join("kubwa.bin");
    assert!(ombi(
        &format!(
            r#""GET", "{base}/kubwa", ChaguoHttp {{ hifadhi: "{}", kikomo: 10 }}"#,
            failed.display()
        ),
        "rejesha j.mwili"
    )
    .contains("kikomo"));
    assert!(!failed.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// HTTPS: a certificate no trusted root signed is refused — an error, not a crash — unless its
/// CA is given (`cheti_ca`); a server asking for a client certificate gets one (`cheti`,
/// `ufunguo`); and HTTPS never redirects to plain HTTP.
#[test]
fn https_roots_client_certificates_and_downgrade() {
    let dir = temp_dir("tls");
    let base = tls_server(&dir, None);
    let ca = dir.join("ca.pem").display().to_string();
    let refused = echo(&format!(r#""GET", "{base}/habari""#));
    assert!(refused.to_lowercase().contains("certificate"), "{refused}");
    assert_eq!(
        echo(&format!(
            r#""GET", "{base}/habari", ChaguoHttp {{ cheti_ca: "{ca}" }}"#
        )),
        "GET /habari|||||"
    );
    assert!(echo(&format!(
        r#""GET", "{base}/elekeza-http", ChaguoHttp {{ cheti_ca: "{ca}" }}"#
    ))
    .contains("kumekataliwa"));

    // Mutual TLS.
    let client_ca_key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let client_ca = params.self_signed(&client_ca_key).unwrap();
    let client_key = rcgen::KeyPair::generate().unwrap();
    let client_cert = rcgen::CertificateParams::new(vec!["mteja".to_string()])
        .unwrap()
        .signed_by(&client_key, &client_ca, &client_ca_key)
        .unwrap();
    let mdir = temp_dir("mtls");
    let mbase = tls_server(&mdir, Some(&client_ca));
    let mca = mdir.join("ca.pem").display().to_string();
    std::fs::write(mdir.join("mteja.pem"), client_cert.pem()).unwrap();
    std::fs::write(mdir.join("mteja.key"), client_key.serialize_pem()).unwrap();
    let without = echo(&format!(
        r#""GET", "{mbase}/a", ChaguoHttp {{ cheti_ca: "{mca}" }}"#
    ));
    assert!(without.starts_with("kosa: "), "{without}");
    assert_eq!(
        echo(&format!(
            r#""GET", "{mbase}/a", ChaguoHttp {{ cheti_ca: "{mca}", cheti: "{}", ufunguo: "{}" }}"#,
            mdir.join("mteja.pem").display(),
            mdir.join("mteja.key").display()
        )),
        "GET /a|||||"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&mdir);
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
