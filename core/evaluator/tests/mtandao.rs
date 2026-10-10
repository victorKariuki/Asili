//! The socket API on `asili-mtandao`: connecting (TCP, Unix, TLS), listeners with `.kubali()`,
//! `.anwani()` and `.simama()`, line and exact reads, half-close, time limits, DNS — and waits
//! inside `sawia` tasks letting the thread's other tasks run.

use std::time::Instant;

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

fn run(body: &str, args: Vec<Value>) -> Value {
    run_function(&compile(body), "jaribu", args).expect("runs")
}

fn text(v: Value) -> String {
    match v {
        Value::Neno(s) => s.to_string(),
        other => panic!("expected Neno, got {other:?}"),
    }
}

/// A server task and a client on one thread: line reads, exact reads, half-close.
#[test]
fn lines_exact_reads_and_half_close_between_tasks() {
    let out = run(
        r#"
        sawia kazi seva(s: MkondoSikilizaji) -> Neno {
            weka m = jaribu (s.kubali())
            weka a = jaribu (m.soma_mstari()).angu("?")
            weka b = jaribu (m.soma_mstari()).angu("?")
            weka cb = jaribu (m.soma_kamili(3))
            weka c = cb.kwa_neno().angu("?")
            weka mwisho = jaribu (m.soma())
            jaribu (m.andika(a + "|" + b + "|" + c + "|" + mwisho))
            weka mbali = m.anwani_mbali()
            m.funga()
            rejesha mbali
        }

        kazi jaribu() -> Neno {
            weka s = jaribu (mkondo_sikiliza("127.0.0.1:0"))
            weka anwani = s.anwani()
            weka kazi_seva = seva(s)
            weka m = jaribu (mkondo_unganisha(anwani))
            jaribu (m.andika("moja\r\nmbili\nabcxyz"))
            jaribu (m.funga_kuandika())
            weka jibu = jaribu (m.soma())
            weka mbali = subiri kazi_seva
            rejesha jibu + " " + ((mbali == m.anwani_yangu()) kama Neno)
        }
    "#,
        vec![],
    );
    assert_eq!(text(out), "moja|mbili|abc|xyz kweli");
}

/// Many clients served by tasks on one thread, each waiting on its socket.
#[test]
fn one_thread_serves_many_connections_concurrently() {
    let start = Instant::now();
    let out = run(
        r#"
        sawia kazi hudumia(m: Mkondo) -> Tupu {
            weka ombi = jaribu (m.soma_mstari()).angu("")
            lala(0.2)
            jaribu (m.andika("jibu " + ombi + "\n"))
        }

        sawia kazi seva(s: MkondoSikilizaji, idadi: Namba) -> Tupu {
            weka zote = []
            kwa i kutoka 0 hadi idadi {
                zote.ongeza(hudumia(jaribu (s.kubali())))
            }
            subiri_zote(zote)
        }

        sawia kazi mteja(anwani: Neno, i: Namba) -> Neno {
            weka m = jaribu (mkondo_unganisha(anwani))
            jaribu (m.andika((i kama Neno) + "\n"))
            rejesha jaribu (m.soma_mstari()).angu("")
        }

        kazi jaribu() -> Neno {
            weka s = jaribu (mkondo_sikiliza("127.0.0.1:0"))
            weka kazi_seva = seva(s, 20)
            weka wateja = []
            kwa i kutoka 0 hadi 20 {
                wateja.ongeza(mteja(s.anwani(), i))
            }
            weka majibu = subiri_zote(wateja)
            subiri kazi_seva
            rejesha (majibu[0] kama Neno) + "," + (majibu[19] kama Neno) + "," + (majibu.urefu() kama Neno)
        }
    "#,
        vec![],
    );
    assert_eq!(text(out), "jibu 0,jibu 19,20");
    // 20 connections that each wait 0.2 s, overlapped.
    assert!(start.elapsed().as_secs_f64() < 2.0, "{:?}", start.elapsed());
}

#[test]
fn unix_sockets_time_limits_and_errors() {
    let path = std::env::temp_dir()
        .join(format!("asili-mtandao-{}.sock", std::process::id()))
        .display()
        .to_string();
    let out = run(
        r#"
        sawia kazi seva(s: MkondoSikilizaji) -> Tupu {
            weka m = jaribu (s.kubali())
            jaribu (m.andika(b"\x00\xffunix"))
        }

        kazi jaribu(njia: Neno) -> Neno {
            weka s = jaribu (mkondo_sikiliza("unix:" + njia))
            weka k = seva(s)
            weka m = jaribu (mkondo_unganisha("unix:" + njia))
            weka b = jaribu (m.soma_kamili(6))
            subiri k

            # Nothing arrives: the time limit ends the read.
            weka s2 = jaribu (mkondo_sikiliza("127.0.0.1:0"))
            weka m2 = jaribu (mkondo_unganisha(s2.anwani()))
            m2.weka_muda(0.05)
            weka kosa = m2.soma_baiti(10).kosa()
            weka kosa2 = mkondo_unganisha("hakuna_mlango").kosa()
            rejesha b.hex() + " " + s.anwani() + " " + kosa + " " + kosa2
        }
    "#,
        vec![Value::neno(path.clone())],
    );
    assert_eq!(
        text(out),
        format!(
            "00ff756e6978 unix:{path} mkondo: muda umekwisha anwani batili (mwenyeji:mlango): hakuna_mlango"
        )
    );
}

/// `.simama()` from another thread ends `mkondo_tumikia`.
#[test]
fn simama_stops_the_worker_pool() {
    let out = run(
        r#"
        kazi hudumia(m: Mkondo) -> Tupu {
            jaribu (m.andika("sawa"))
        }

        kazi simamisha(s: MkondoSikilizaji) -> Tupu {
            weka m = jaribu (mkondo_unganisha(s.anwani()))
            jaribu (m.soma())
            s.simama()
        }

        kazi jaribu() -> Neno {
            weka s = jaribu (mkondo_sikiliza("127.0.0.1:0"))
            weka uzi = jaribu (tenda("simamisha", s))
            jaribu (mkondo_tumikia(s, "hudumia", 3))
            jaribu (subiri_tenda(uzi))
            rejesha "imesimama"
        }
    "#,
        vec![],
    );
    assert_eq!(text(out), "imesimama");
}

#[test]
fn tls_client_with_a_private_root_and_dns() {
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let dir = std::env::temp_dir();
    let cert = dir.join(format!("asili-mtandao-cert-{}.pem", std::process::id()));
    let key = dir.join(format!("asili-mtandao-key-{}.pem", std::process::id()));
    std::fs::write(&cert, ck.cert.pem()).unwrap();
    std::fs::write(&key, ck.key_pair.serialize_pem()).unwrap();
    let (cert, key) = (cert.display().to_string(), key.display().to_string());

    let out = run(
        r#"
        kazi hudumia(m: Mkondo) -> Tupu {
            weka ombi = jaribu (m.soma_mstari()).angu("")
            jaribu (m.andika("salama: " + ombi + "\n"))
        }

        kazi seva(s: MkondoSikilizaji, cheti: Neno, ufunguo: Neno) -> Tupu {
            weka tls = jaribu (tls_sanidi(cheti, ufunguo))
            jaribu (mkondo_tumikia(s, "hudumia", 1, Chaguo::Kuna(tls)))
        }

        kazi jaribu(cheti: Neno, ufunguo: Neno) -> Neno {
            weka s = jaribu (mkondo_sikiliza("127.0.0.1:0"))
            weka mlango = s.anwani().gawanya(":")[1]
            weka uzi = jaribu (tenda("seva", s, cheti.clona(), ufunguo))
            weka m = jaribu (mkondo_unganisha("localhost:" + mlango, ChaguoMkondo {
                tls: kweli,
                cheti_ca: cheti,
                muda: 5,
                familia_ip: 4,
            }))
            jaribu (m.andika("habari\n"))
            weka jibu = jaribu (m.soma_mstari()).angu("")

            # The Mozilla roots do not trust a self-signed certificate.
            weka kosa = mkondo_unganisha("localhost:" + mlango, ChaguoMkondo { tls: kweli, familia_ip: 4 }).kosa().kata(0, 3)
            s.simama()
            jaribu (subiri_tenda(uzi))
            weka ips = jaribu (tafuta_anwani("localhost"))
            rejesha jibu + " " + kosa + " " + ((ips.urefu() > 0) kama Neno)
        }
    "#,
        vec![Value::neno(cert), Value::neno(key)],
    );
    assert_eq!(text(out), "salama: habari TLS kweli");
}
