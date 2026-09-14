//! Mkondo (TCP stream handle): mkondo_unganisha/.soma()/.andika()/.funga(). Available via the
//! existing `leta mfumo` gate, not an opt-in module (see docs/design/faili-mkondo-design.md).

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, extern_env_from_imports, Module};
use std::io::Read;
use std::net::TcpListener;
use std::thread;

fn compile(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    semantic_check_with_env(&module, false, fns, consts).expect("semantic check");
    module
}

#[test]
fn connect_write_and_server_receives_it() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).expect("read");
        String::from_utf8_lossy(&buf).to_string()
    });

    let src = format!(
        r#"
        leta mfumo

        kazi jaribu() -> Tupu {{
            weka m = jaribu (mkondo_unganisha("{}"))
            jaribu (m.andika("habar"))
            m.funga()
        }}
    "#,
        addr
    );
    let module = compile(&src);
    run_function(&module, "jaribu", vec![]).expect("runs");
    let received = server.join().expect("server thread");
    assert_eq!(received, "habar");
}

#[test]
fn connecting_to_a_closed_port_returns_kosa() {
    // Bind then immediately drop to get a very-likely-unused port with nothing listening.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);

    let src = format!(
        r#"
        leta mfumo

        kazi jaribu() -> Ukweli {{
            linganisha mkondo_unganisha("{}") {{
                Tokeo::Sawa(_) => {{ rejesha si_kweli }}
                Tokeo::Kosa(_) => {{ rejesha kweli }}
            }}
        }}
    "#,
        addr
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn double_funga_is_a_safe_no_op() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf);
    });

    let src = format!(
        r#"
        leta mfumo

        kazi jaribu() -> Tupu {{
            weka m = jaribu (mkondo_unganisha("{}"))
            m.funga()
            m.funga()
        }}
    "#,
        addr
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]);
    assert!(result.is_ok(), "closing an already-closed Mkondo handle must not panic");
    let _ = server.join();
}

#[test]
fn soma_bailisi_reads_a_bounded_chunk_without_waiting_for_eof() {
    // The whole point of .soma_bailisi over .soma(): the peer never closes/half-closes its
    // write side here, so .soma()'s read_to_string would block forever — .soma_bailisi must
    // still return as soon as the requested bytes are available.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        use std::io::Write;
        stream.write_all(b"habari").expect("write");
        // Deliberately no shutdown/close here — the connection stays open exactly like a real
        // HTTP/1.1 keep-alive peer that intends to send more later.
        thread::sleep(std::time::Duration::from_millis(200));
    });

    let src = format!(
        r#"
        leta mfumo

        kazi jaribu() -> Neno {{
            weka m = jaribu (mkondo_unganisha("{}"))
            rejesha jaribu (m.soma_bailisi(6))
        }}
    "#,
        addr
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("habari".to_string()));
    let _ = server.join();
}

#[test]
fn soma_bailisi_on_closed_handle_returns_kosa() {
    let src = r#"
        leta mfumo

        kazi jaribu() -> Ukweli {
            weka m = jaribu (mkondo_unganisha("127.0.0.1:1"))
            m.funga()
            linganisha m.soma_bailisi(10) {
                Tokeo::Kosa(_) => { rejesha kweli }
                Tokeo::Sawa(_) => { rejesha si_kweli }
            }
        }
    "#;
    // mkondo_unganisha itself will fail to connect to a closed port before we even get to
    // .funga()/.soma_bailisi — this test instead needs a real connection first. Use a
    // throwaway local listener so the connect succeeds, then close and check soma_bailisi.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let _server = thread::spawn(move || {
        let _ = listener.accept();
    });
    let src = src.replacen("127.0.0.1:1", &addr.to_string(), 1);
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), "soma_bailisi on a closed handle must return Kosa, not panic");
}

#[test]
fn mkondo_requires_leta_mfumo_or_resolved_module() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka m = mkondo_unganisha("127.0.0.1:1")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    assert!(result.is_err(), "mkondo_unganisha should be unknown without `leta mfumo`");
}
