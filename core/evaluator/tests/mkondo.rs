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
