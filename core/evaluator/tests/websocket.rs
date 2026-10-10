//! WebSockets from Asili programs: `ws_unganisha` on the client, and a server that answers an
//! upgrade request with `JibuHttp { ws: ... }`, whose function receives each connection as a
//! `MkondoWs`.

use std::net::TcpStream;
use std::time::Duration;

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{extern_env_from_imports, parse_tokens, semantic_check_with_env, Module};

const PRELUDE: &str = r#"
    leta mfumo
    leta sambamba
    leta majira
    leta matumizi
"#;

fn compile(body: &str) -> Module {
    let src = format!("{PRELUDE}\n{body}");
    let toks = tokenize(&src).expect("tokenize");
    let module = parse_tokens(&toks).unwrap_or_else(|e| panic!("parse: {e:?}"));
    let (fns, consts) = extern_env_from_imports(&module);
    if let Err(e) = semantic_check_with_env(&module, false, fns, consts) {
        panic!("semantic check: {e:?}");
    }
    module
}

const SERVER: &str = r#"
    kazi mwangwi(ws: MkondoWs) -> Tupu {
        wakati kweli {
            weka m = jaribu (ws.pokea_neno())
            linganisha m {
                Chaguo::Kuna(maandishi) => { jaribu (ws.tuma("echo: " + maandishi)) }
                Chaguo::Hamna => { ws.funga() }
            }
        }
    }

    kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
        ikiwa ombi.sehemu == "/ws" {
            rejesha JibuHttp { hali: 101, vichwa: kamusi(), mwili: "", ws: "mwangwi" }
        }
        rejesha JibuHttp { hali: 200, vichwa: kamusi(), mwili: "sawa" }
    }

    kazi simamisha(s: MkondoSikilizaji, udhibiti: MkondoSikilizaji) -> Tupu {
        weka _m = jaribu (udhibiti.kubali())
        s.simama()
    }

    kazi anza(anwani: Neno, anwani_ya_udhibiti: Neno) -> Neno {
        weka s = jaribu (mkondo_sikiliza(anwani))
        weka udhibiti = jaribu (mkondo_sikiliza(anwani_ya_udhibiti))
        weka uzi = jaribu (tenda("simamisha", s, udhibiti))
        jaribu (mkondo_tumikia_http(s, "mtumishi", 2, Chaguo::Hamna, ChaguoSeva { muda_kuzima: 1 }))
        jaribu (subiri_tenda(uzi))
        rejesha "imesimama"
    }
"#;

fn free_port() -> String {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    probe.local_addr().unwrap().to_string()
}

fn wait_for(addr: &str) {
    for _ in 0..200 {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("server never came up at {addr}");
}

fn text(v: Value) -> String {
    match v {
        Value::Neno(s) => s.to_string(),
        other => panic!("expected Neno, got {other:?}"),
    }
}

#[test]
fn websocket_client_and_server_in_asili() {
    let (addr, control) = (free_port(), free_port());
    let server = {
        let (addr, control) = (addr.clone(), control.clone());
        std::thread::spawn(move || {
            run_function(
                &compile(SERVER),
                "anza",
                vec![Value::neno(addr), Value::neno(control)],
            )
            .map(|v| format!("{v:?}"))
            .map_err(|e| format!("{e:?}"))
        })
    };
    wait_for(&addr);

    let client = compile(&format!(
        r#"
        kazi jaribu(url: Neno) -> Neno {{
            weka ws = jaribu (ws_unganisha(url))
            jaribu (ws.tuma("habari"))
            weka a = jaribu (ws.pokea_neno()).angu("")
            jaribu (ws.tuma_baiti(b"\x01\x02"))
            weka b = jaribu (ws.pokea()).angu(b"")
            ws.funga()
            rejesha a + " " + (b.urefu() kama Neno)
        }}
    "#
    ));
    let out = run_function(
        &client,
        "jaribu",
        vec![Value::neno(format!("ws://{addr}/ws"))],
    )
    .expect("runs");
    // The server echoes each message as text: "echo: " plus the two control bytes sent.
    assert_eq!(text(out), "echo: habari 8");

    let stopper = TcpStream::connect(&control).expect("control");
    drop(stopper);
    let out = server.join().unwrap().expect("server runs");
    assert!(out.contains("imesimama"), "{out}");
}
