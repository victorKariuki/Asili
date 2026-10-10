//! The HTTP server on hyper (`mkondo_tumikia_http`): the new `OmbiHttp` fields, requests
//! answered concurrently as `sawia` tasks, streamed bodies and server-sent events from a channel,
//! limits (`ChaguoSeva`), and stopping with `.simama()`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{extern_env_from_imports, parse_tokens, semantic_check_with_env, Module};

const SERVER: &str = r#"
    leta mfumo
    leta sambamba
    leta majira
    leta matumizi

    sawia kazi matukio(tx: NjiaTx<Neno>) -> Tupu {
        kwa i kutoka 0 hadi 3 {
            jaribu (tx.tuma(tukio_sse("tukio " + (i kama Neno), "hesabu", i kama Neno)))
            lala(0.01)
        }
    }

    kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
        ikiwa ombi.sehemu == "/polepole" {
            lala(0.3)
            rejesha JibuHttp { hali: 200, vichwa: kamusi(), mwili: "polepole" }
        }
        ikiwa ombi.sehemu == "/matukio" {
            weka p = njia()
            matukio(p.kwanza())
            weka vichwa = kamusi()
            vichwa.ingiza("content-type", "text/event-stream")
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: "", mwili_njia: p.pili() }
        }
        weka jina = ombi.hoja.pata("jina").angu("?")
        weka maelezo = ombi.sehemu + " " + jina + " " + ombi.toleo + " " + (ombi.mwili_baiti.urefu() kama Neno)
        rejesha JibuHttp { hali: 200, vichwa: kamusi(), mwili: maelezo, sababu: "Sawa Kabisa" }
    }

    # Stops the server once something connects to the control listener.
    kazi simamisha(s: MkondoSikilizaji, udhibiti: MkondoSikilizaji) -> Tupu {
        weka _m = jaribu (udhibiti.kubali())
        s.simama()
    }

    kazi anza(anwani: Neno, anwani_ya_udhibiti: Neno) -> Neno {
        weka s = jaribu (mkondo_sikiliza(anwani))
        weka udhibiti = jaribu (mkondo_sikiliza(anwani_ya_udhibiti))
        weka uzi = jaribu (tenda("simamisha", s, udhibiti))
        jaribu (mkondo_tumikia_http(s, "mtumishi", 1, Chaguo::Hamna, ChaguoSeva { kikomo_mwili: 10, muda_kuzima: 1 }))
        jaribu (subiri_tenda(uzi))
        rejesha "imesimama"
    }
"#;

fn compile() -> Module {
    let toks = tokenize(SERVER).expect("tokenize");
    let module = parse_tokens(&toks).unwrap_or_else(|e| panic!("parse: {e:?}"));
    let (fns, consts) = extern_env_from_imports(&module);
    if let Err(e) = semantic_check_with_env(&module, false, fns, consts) {
        panic!("semantic check: {e:?}");
    }
    module
}

fn exchange(addr: &str, request: &str) -> String {
    let mut s = TcpStream::connect(addr).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    s.write_all(request.as_bytes()).expect("write");
    let mut out = Vec::new();
    let _ = s.read_to_end(&mut out);
    String::from_utf8_lossy(&out).into_owned()
}

fn get(addr: &str, path: &str) -> String {
    exchange(
        addr,
        &format!("GET {path} HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n"),
    )
}

fn free_port() -> String {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    probe.local_addr().unwrap().to_string()
}

fn connect_with_retry(addr: &str) -> TcpStream {
    for _ in 0..200 {
        if let Ok(s) = TcpStream::connect(addr) {
            return s;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("server never came up at {addr}");
}

#[test]
fn requests_streams_limits_concurrency_and_stop() {
    let module = compile();
    let (addr, control) = (free_port(), free_port());
    let server = {
        let (addr, control) = (addr.clone(), control.clone());
        std::thread::spawn(move || {
            run_function(
                &module,
                "anza",
                vec![Value::neno(addr), Value::neno(control)],
            )
            .map(|v| format!("{v:?}"))
            .map_err(|e| format!("{e:?}"))
        })
    };
    drop(connect_with_retry(&addr));

    let r = get(&addr, "/a%20b?jina=Juma");
    assert!(r.starts_with("HTTP/1.1 200 Sawa Kabisa\r\n"), "{r}");
    assert!(r.ends_with("/a b Juma HTTP/1.1 0"), "{r}");

    let r = exchange(
        &addr,
        "POST / HTTP/1.1\r\nhost: h\r\ncontent-length: 20\r\nconnection: close\r\n\r\n01234567890123456789",
    );
    assert!(r.starts_with("HTTP/1.1 413"), "{r}");

    let r = get(&addr, "/matukio");
    assert!(r.contains("content-type: text/event-stream"), "{r}");
    for i in 0..3 {
        assert!(
            r.contains(&format!("id: {i}\nevent: hesabu\ndata: tukio {i}\n\n")),
            "{r}"
        );
    }

    // One worker thread: two slow requests overlap.
    let start = Instant::now();
    let slow: Vec<_> = (0..2)
        .map(|_| {
            let addr = addr.clone();
            std::thread::spawn(move || get(&addr, "/polepole"))
        })
        .collect();
    for t in slow {
        assert!(t.join().unwrap().ends_with("polepole"));
    }
    assert!(
        start.elapsed() < Duration::from_millis(550),
        "{:?}",
        start.elapsed()
    );

    drop(connect_with_retry(&control));
    let out = server.join().unwrap().expect("runs");
    assert!(out.contains("imesimama"), "{out}");
}
