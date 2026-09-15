//! HTTP/1.1 request/response framing: mkondo_tumikia_http. See
//! docs/design/http-framing-design.md. Same role-flip pattern as mkondo_sikiliza.rs — the Asili
//! program is the server, driven by real raw HTTP bytes written/read directly over a
//! std::net::TcpStream from the test (no HTTP client crate needed: writing/reading the exact
//! wire bytes is both simpler and a stronger proof than trusting a client library's own framing).
//!
//! `OmbiHttp`/`JibuHttp` are plain `Value::Struct`s at runtime (the same reflection-friendly
//! shape the JSON codec uses) — but the Asili *source* in each test still needs a matching
//! `umbo OmbiHttp { ... }`/`umbo JibuHttp { ... }` declaration for the semantic analyzer to
//! accept `ombi: OmbiHttp` as a parameter type and type-check field access on it (SEM098/SEM099
//! otherwise) — the struct's *name* is never checked at runtime by `http.rs`, only its fields.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, extern_env_from_imports, Module};

fn compile(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    semantic_check_with_env(&module, false, fns, consts).expect("semantic check");
    module
}

const STRUCTS: &str = r#"
umbo OmbiHttp { njia: Neno, anwani: Neno, vichwa: Kamusi<Neno, Neno>, mwili: Neno }
umbo JibuHttp { hali: Namba, vichwa: Kamusi<Neno, Neno>, mwili: Neno }
"#;

fn start_server(kazi_and_umbo_src: &str, idadi_ya_nyuzi: f64) -> String {
    let src = format!(
        r#"
        leta mfumo
        {STRUCTS}
        {kazi_and_umbo_src}

        kazi anza(anwani: Neno) -> Tupu {{
            weka s = jaribu (mkondo_sikiliza(anwani))
            jaribu (mkondo_tumikia_http(s, "mtumishi", {idadi_ya_nyuzi}))
        }}
    "#
    );
    let module = compile(&src);

    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let addr_clone = addr.clone();
    std::thread::spawn(move || {
        let _ = run_function(&module, "anza", vec![Value::Neno(addr_clone)]);
    });
    addr
}

fn connect_with_retry(addr: &str) -> TcpStream {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match TcpStream::connect(addr) {
            Ok(s) => return s,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => panic!("could not connect to {addr}: {e}"),
        }
    }
}

/// Reads exactly one HTTP response off `stream` (status line + headers + Content-Length body),
/// leaving the stream open for a possible next response — mirrors what a real keep-alive client
/// does, and is what makes the keep-alive test below possible.
fn read_one_response(stream: &mut TcpStream) -> (u16, String) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        // Look for the header/body boundary first.
        if let Some(header_end) = find_double_crlf(&buf) {
            let header_text = String::from_utf8_lossy(&buf[..header_end]);
            let status: u16 = header_text
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|s| s.parse().ok())
                .expect("status line");
            let content_length: usize = header_text
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.trim().eq_ignore_ascii_case("content-length") {
                        v.trim().parse().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            let body_start = header_end;
            while buf.len() < body_start + content_length {
                let n = stream.read(&mut chunk).expect("read body");
                assert!(n > 0, "connection closed mid-body");
                buf.extend_from_slice(&chunk[..n]);
            }
            let body = String::from_utf8_lossy(&buf[body_start..body_start + content_length]).into_owned();
            return (status, body);
        }
        let n = stream.read(&mut chunk).expect("read");
        assert!(n > 0, "connection closed before headers completed");
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

#[test]
fn get_request_response_round_trips() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: "njia=" + ombi.njia + " anwani=" + ombi.anwani }
        }
    "#;
    let addr = start_server(kazi, 2.0);

    let mut client = connect_with_retry(&addr);
    client
        .write_all(b"GET /habari HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("write request");

    let (status, body) = read_one_response(&mut client);
    assert_eq!(status, 200);
    assert_eq!(body, "njia=GET anwani=/habari");
}

#[test]
fn request_headers_and_body_reach_kazi_jina() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            weka aina = jaribu (ombi.vichwa.pata("X-Aina"))
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: (aina kama Neno) + "|" + ombi.mwili }
        }
    "#;
    let addr = start_server(kazi, 2.0);

    let mut client = connect_with_retry(&addr);
    let body = "data-ya-ombi";
    let request = format!(
        "POST /tuma HTTP/1.1\r\nHost: localhost\r\nX-Aina: jaribio\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    client.write_all(request.as_bytes()).expect("write request");

    let (status, response_body) = read_one_response(&mut client);
    assert_eq!(status, 200);
    assert_eq!(response_body, "jaribio|data-ya-ombi");
}

#[test]
fn keep_alive_serves_two_requests_on_one_connection() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: ombi.anwani }
        }
    "#;
    let addr = start_server(kazi, 1.0);

    let mut client = connect_with_retry(&addr);
    // No `Connection: close` here — HTTP/1.1 defaults to keep-alive, so the same connection
    // must still be usable for a second request afterward.
    client
        .write_all(b"GET /kwanza HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("write first request");
    let (status1, body1) = read_one_response(&mut client);
    assert_eq!(status1, 200);
    assert_eq!(body1, "/kwanza");

    client
        .write_all(b"GET /pili HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("write second request");
    let (status2, body2) = read_one_response(&mut client);
    assert_eq!(status2, 200);
    assert_eq!(body2, "/pili", "second request over the same connection must still be answered");
}

/// Issue #20: a `Transfer-Encoding: chunked` request body is decoded for real (not rejected with
/// 501) — a single chunk plus the terminating zero-size chunk.
#[test]
fn chunked_transfer_encoding_request_body_is_decoded() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: ombi.mwili }
        }
    "#;
    let addr = start_server(kazi, 1.0);

    let mut client = connect_with_retry(&addr);
    client
        .write_all(b"POST /pakia HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n4\r\ntest\r\n0\r\n\r\n")
        .expect("write chunked request");

    let (status, body) = read_one_response(&mut client);
    assert_eq!(status, 200);
    assert_eq!(body, "test", "the decoded (unchunked) body must reach kazi_jina");
}

/// Multiple chunks of different sizes, plus a chunk-size line carrying an extension (`;ext`,
/// which real clients sometimes send and which must be ignored, not fail hex parsing) — proves
/// this isn't just a single-chunk special case.
#[test]
fn chunked_request_with_multiple_chunks_and_extension_is_decoded() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: ombi.mwili }
        }
    "#;
    let addr = start_server(kazi, 1.0);

    let mut client = connect_with_retry(&addr);
    // "hello" (5 bytes) + ";ignored-ext" on the size line + " world" (6 bytes) + terminator,
    // with a trailer header thrown in to prove trailers are consumed without breaking framing.
    client
        .write_all(
            b"POST /pakia HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n\
              5;ignored-ext\r\nhello\r\n6\r\n world\r\n0\r\nX-Trailer: baadaye\r\n\r\n",
        )
        .expect("write chunked request");

    let (status, body) = read_one_response(&mut client);
    assert_eq!(status, 200);
    assert_eq!(body, "hello world");
}

/// Pipelining (issue #21): two full requests written in a single `write_all` call — no
/// interleaved read between them, so the second request's bytes necessarily arrive in the same
/// `stream.read` the server uses to complete the first request. Both must still be answered, in
/// order, on the one connection.
#[test]
fn pipelined_requests_are_both_answered_in_order() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: ombi.anwani }
        }
    "#;
    let addr = start_server(kazi, 1.0);

    let mut client = connect_with_retry(&addr);
    let first = b"GET /kwanza HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec();
    let second = b"GET /pili HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n".to_vec();
    let mut both = first;
    both.extend_from_slice(&second);
    client.write_all(&both).expect("write both pipelined requests at once");

    let (status1, body1) = read_one_response(&mut client);
    assert_eq!(status1, 200);
    assert_eq!(body1, "/kwanza", "first pipelined request must be answered first");

    let (status2, body2) = read_one_response(&mut client);
    assert_eq!(status2, 200);
    assert_eq!(body2, "/pili", "second pipelined request must still be answered, in order");
}

/// 100-continue (issue #22): a client sending `Expect: 100-continue` must receive a real
/// intermediate `HTTP/1.1 100 Continue\r\n\r\n` before the final response — proven by reading the
/// interim status line directly, before the body is even written.
#[test]
fn expect_100_continue_gets_an_intermediate_response() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: ombi.mwili }
        }
    "#;
    let addr = start_server(kazi, 1.0);

    let mut client = connect_with_retry(&addr);
    let body = "payload";
    let request = format!(
        "POST /pakia HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    client.write_all(request.as_bytes()).expect("write request headers");

    // Read exactly the interim response line before sending the body — a real client waiting
    // for 100-continue would do the same.
    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    while find_double_crlf(&buf).is_none() {
        let n = client.read(&mut chunk).expect("read interim response");
        assert!(n > 0, "connection closed before 100 Continue arrived");
        buf.extend_from_slice(&chunk[..n]);
    }
    let interim = String::from_utf8_lossy(&buf);
    assert!(interim.starts_with("HTTP/1.1 100"), "expected an interim 100 Continue, got: {interim}");

    client.write_all(body.as_bytes()).expect("write body after 100 Continue");

    let (status, response_body) = read_one_response(&mut client);
    assert_eq!(status, 200);
    assert_eq!(response_body, body);
}

#[test]
fn connection_closed_mid_body_is_rejected_with_400_not_a_hang() {
    // Declares a body longer than what's actually sent, then closes the write half — the
    // server's bounded-read loop (read_request) sees EOF (Ok(0)) with an incomplete request
    // already buffered, which the plan calls out as a real, non-timeout-based error path
    // distinct from a merely slow/stalled peer: the peer here has definitively finished sending
    // (it closed), so there's nothing to wait for at all — this must resolve immediately, not
    // deadlock or wait for a timeout that will never help.
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: ombi.mwili }
        }
    "#;
    let addr = start_server(kazi, 1.0);

    let mut client = connect_with_retry(&addr);
    client
        .write_all(b"POST /haujakamilika HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\nfupi")
        .expect("write incomplete request");
    client.shutdown(std::net::Shutdown::Write).expect("shutdown write half");

    let (status, _) = read_one_response(&mut client);
    assert_eq!(status, 400, "an incomplete body followed by connection close must be rejected immediately");
}

#[test]
fn malformed_request_line_is_rejected_with_400() {
    let kazi = r#"
        kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
            weka vichwa = kamusi()
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: ombi.mwili }
        }
    "#;
    let addr = start_server(kazi, 1.0);

    let mut client = connect_with_retry(&addr);
    client.write_all(b"SI_HTTP KABISA\r\n\r\n").expect("write garbage");

    let (status, _) = read_one_response(&mut client);
    assert_eq!(status, 400);
}
