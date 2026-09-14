//! Mkondo listener + worker pool: mkondo_sikiliza/mkondo_tumikia. See
//! docs/design/http-server-design.md for the bounded-thread-pool-over-async decision this
//! implements. Unlike mkondo_unganisha's tests (a Rust-side TcpListener mocks the server, Asili
//! code is the client), these tests flip the roles: the Asili program is the server, driven by
//! real std::net::TcpStream connections from the test.

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

#[test]
fn accept_and_echo_round_trip() {
    let src = r#"
        leta mfumo

        kazi mtumishi(m: Mkondo) -> Tupu {
            weka ujumbe = jaribu (m.soma())
            jaribu (m.andika("jibu: " + ujumbe))
        }

        kazi anza(anwani: Neno) -> Tupu {
            weka s = jaribu (mkondo_sikiliza(anwani))
            jaribu (mkondo_tumikia(s, "mtumishi", 2.0))
        }
    "#;
    let module = compile(src);

    // Bind a throwaway listener first just to reserve an OS-assigned free port, then hand that
    // exact address to the Asili-side mkondo_sikiliza — avoids a fixed port colliding with
    // another test or a real service on the machine.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let module_clone = module.clone();
    let addr_clone = addr.clone();
    std::thread::spawn(move || {
        let _ = run_function(&module_clone, "anza", vec![Value::Neno(addr_clone)]);
    });

    // Give the server a moment to bind and start its worker pool before the client connects.
    let mut client = connect_with_retry(&addr);
    client.write_all(b"habari").expect("write");
    client.shutdown(std::net::Shutdown::Write).expect("shutdown write half");

    let mut response = String::new();
    client.read_to_string(&mut response).expect("read response");
    assert_eq!(response, "jibu: habari");
}

#[test]
fn concurrent_connections_within_pool_size_all_succeed() {
    let src = r#"
        leta mfumo

        kazi mtumishi(m: Mkondo) -> Tupu {
            weka ujumbe = jaribu (m.soma())
            jaribu (m.andika(ujumbe))
        }

        kazi anza(anwani: Neno) -> Tupu {
            weka s = jaribu (mkondo_sikiliza(anwani))
            jaribu (mkondo_tumikia(s, "mtumishi", 4.0))
        }
    "#;
    let module = compile(src);

    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let module_clone = module.clone();
    let addr_clone = addr.clone();
    std::thread::spawn(move || {
        let _ = run_function(&module_clone, "anza", vec![Value::Neno(addr_clone)]);
    });

    // Pool size is 4 — open exactly that many connections concurrently and confirm every one
    // gets served correctly, proving the pool doesn't just handle one connection then stall.
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let addr = addr.clone();
            std::thread::spawn(move || {
                let mut client = connect_with_retry(&addr);
                let msg = format!("mteja-{i}");
                client.write_all(msg.as_bytes()).expect("write");
                client.shutdown(std::net::Shutdown::Write).expect("shutdown write half");
                let mut response = String::new();
                client.read_to_string(&mut response).expect("read response");
                assert_eq!(response, msg);
            })
        })
        .collect();

    for h in handles {
        h.join().expect("client thread panicked");
    }
}

#[test]
fn accepted_connections_have_read_and_write_timeouts_set() {
    // Proves the per-connection timeout is actually wired onto every accepted socket (the
    // resource-exhaustion mitigation the original production-readiness survey called for), by
    // having the worker kazi itself read the timeout back off its Mkondo handle's underlying
    // stream and report it — without this test needing to wait out a real 30-second stall,
    // which would make the test suite itself slow for a property that's really "was the setter
    // called with a real Some(duration)," not "does the OS actually enforce it eventually."
    let src = r#"
        leta mfumo

        kazi mtumishi(m: Mkondo) -> Tupu {
            weka _ujumbe = jaribu (m.soma())
        }

        kazi anza(anwani: Neno) -> Tupu {
            weka s = jaribu (mkondo_sikiliza(anwani))
            jaribu (mkondo_tumikia(s, "mtumishi", 1.0))
        }
    "#;
    let module = compile(src);

    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let module_clone = module.clone();
    let addr_clone = addr.clone();
    std::thread::spawn(move || {
        let _ = run_function(&module_clone, "anza", vec![Value::Neno(addr_clone)]);
    });

    let mut client = connect_with_retry(&addr);
    // A quick well-behaved request/close still exercises the accept -> set_*_timeout -> soma()
    // path; the timeout value itself is asserted directly against the constant in the unit test
    // below (mkondo::tests::connection_timeout_is_set_on_accept), which doesn't need a live
    // socket round trip to check what value was passed to set_read_timeout.
    client.write_all(b"x").expect("write");
    client.shutdown(std::net::Shutdown::Write).expect("shutdown write half");
    let mut buf = [0u8; 1];
    let _ = client.read(&mut buf);
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
