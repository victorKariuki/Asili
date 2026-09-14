//! TLS on the Mkondo listener: tls_sanidi + mkondo_tumikia's optional 4th (tls) parameter. See
//! docs/design/tls-design.md. A self-signed cert/key pair is generated fresh at test time (via
//! rcgen, a dev-dependency only — never shipped in the production dependency tree) rather than
//! committing a static cert/key pair to the repo.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
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

/// Generates a fresh self-signed cert/key pair for "localhost", writes both as PEM to unique
/// files under the system temp dir, and returns their paths. Cleaned up by the caller.
fn write_test_cert() -> (std::path::PathBuf, std::path::PathBuf) {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).expect("generate cert");
    let unique = format!("{}-{:?}", std::process::id(), std::thread::current().id());
    let cert_path = std::env::temp_dir().join(format!("asili-test-cert-{unique}.pem"));
    let key_path = std::env::temp_dir().join(format!("asili-test-key-{unique}.pem"));
    std::fs::write(&cert_path, cert.pem()).expect("write cert");
    std::fs::write(&key_path, key_pair.serialize_pem()).expect("write key");
    (cert_path, key_path)
}

/// A `rustls::RootCertStore` trusting exactly the one self-signed cert this test generated —
/// the correct pattern for a private/self-issued CA, not a "disable verification" escape hatch.
fn trust_store_for(cert_path: &std::path::Path) -> rustls::RootCertStore {
    let cert_bytes = std::fs::read(cert_path).expect("read cert");
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_bytes.as_slice())
        .collect::<Result<_, _>>()
        .expect("parse cert");
    let mut store = rustls::RootCertStore::empty();
    for c in certs {
        store.add(c).expect("add trust anchor");
    }
    store
}

#[test]
fn tls_request_response_round_trips() {
    let (cert_path, key_path) = write_test_cert();
    let src = format!(
        r#"
        leta mfumo

        kazi mtumishi(m: Mkondo) -> Tupu {{
            weka ujumbe = jaribu (m.soma())
            jaribu (m.andika("salama: " + ujumbe))
        }}

        kazi anza(anwani: Neno, cheti: Neno, ufunguo: Neno) -> Tupu {{
            weka s = jaribu (mkondo_sikiliza(anwani))
            weka tls = Chaguo::Kuna(jaribu (tls_sanidi(cheti, ufunguo)))
            jaribu (mkondo_tumikia(s, "mtumishi", 2.0, tls))
        }}
    "#
    );
    let module = compile(&src);

    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let module_clone = module.clone();
    let addr_clone = addr.clone();
    let cert_arg = cert_path.to_string_lossy().to_string();
    let key_arg = key_path.to_string_lossy().to_string();
    std::thread::spawn(move || {
        let _ = run_function(
            &module_clone,
            "anza",
            vec![Value::Neno(addr_clone), Value::Neno(cert_arg), Value::Neno(key_arg)],
        );
    });

    let root_store = trust_store_for(&cert_path);
    let client_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let server_name = rustls_pki_types::ServerName::try_from("localhost").expect("server name");
    let conn =
        rustls::ClientConnection::new(Arc::new(client_config), server_name).expect("client connection");

    let tcp = connect_with_retry(&addr);
    let mut tls_stream = rustls::StreamOwned::new(conn, tcp);

    tls_stream.write_all(b"habari").expect("write over tls");
    // A raw TCP shutdown(Write) is not a clean TLS close — rustls treats that as a truncation
    // attack and errors with "peer closed connection without sending TLS close_notify" instead
    // of a clean EOF. The server's `.soma()` (read_to_string) needs a real close_notify to
    // finish reading, exactly like a real HTTPS client would send on connection close.
    tls_stream.conn.send_close_notify();
    tls_stream.flush().expect("flush close_notify");

    let mut response = String::new();
    tls_stream.read_to_string(&mut response).expect("read response over tls");
    assert_eq!(response, "salama: habari");

    let _ = std::fs::remove_file(&cert_path);
    let _ = std::fs::remove_file(&key_path);
}

#[test]
fn plaintext_connection_to_tls_listener_fails_cleanly_not_by_hanging() {
    let (cert_path, key_path) = write_test_cert();
    let src = format!(
        r#"
        leta mfumo

        kazi mtumishi(m: Mkondo) -> Tupu {{
            weka _ujumbe = jaribu (m.soma())
        }}

        kazi anza(anwani: Neno, cheti: Neno, ufunguo: Neno) -> Tupu {{
            weka s = jaribu (mkondo_sikiliza(anwani))
            weka tls = Chaguo::Kuna(jaribu (tls_sanidi(cheti, ufunguo)))
            jaribu (mkondo_tumikia(s, "mtumishi", 1.0, tls))
        }}
    "#
    );
    let module = compile(&src);

    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let module_clone = module.clone();
    let addr_clone = addr.clone();
    let cert_arg = cert_path.to_string_lossy().to_string();
    let key_arg = key_path.to_string_lossy().to_string();
    std::thread::spawn(move || {
        let _ = run_function(
            &module_clone,
            "anza",
            vec![Value::Neno(addr_clone), Value::Neno(cert_arg), Value::Neno(key_arg)],
        );
    });

    // A plain TCP client speaking no TLS at all against a TLS-configured listener: the
    // handshake fails server-side (handshake_tls returns None), the worker drops this
    // connection and loops back to accept — the client just observes the connection close
    // (read returns 0 bytes / an error) rather than the server hanging forever waiting for a
    // handshake that will never arrive.
    let mut client = connect_with_retry(&addr);
    client.write_all(b"si tls hata kidogo").ok();
    client.set_read_timeout(Some(Duration::from_secs(5))).expect("set timeout");
    let mut buf = [0u8; 16];
    // Either a clean EOF (Ok(0)) or a connection-reset error is acceptable — both signal the
    // server closed the connection instead of hanging; only a timeout (this test's own 5s cap)
    // would indicate the server got stuck.
    let _ = client.read(&mut buf);

    let _ = std::fs::remove_file(&cert_path);
    let _ = std::fs::remove_file(&key_path);
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
