//! HTTP/3: a server answering on UDP beside its TCP listener, and the client fetching from it.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use asili_mtandao::http::h3;
use asili_mtandao::http::seva::{serve, Handler, Jibu, JibuMwili, Ombi, ServerOptions};
use asili_mtandao::{bind, tls, BindOptions};

fn handler() -> Handler {
    Rc::new(|o: Ombi| {
        Box::pin(async move {
            Jibu {
                status: 201,
                reason: None,
                headers: vec![("x-toleo".into(), o.version.into())],
                body: JibuMwili::Full(
                    format!(
                        "{} {} {}",
                        o.method,
                        o.path,
                        String::from_utf8_lossy(&o.body)
                    )
                    .into_bytes(),
                ),
                websocket: None,
            }
        })
    })
}

#[test]
fn http3_request_and_response() {
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let dir = std::env::temp_dir();
    let (cert, key) = (
        dir.join(format!("h3-cert-{}.pem", std::process::id())),
        dir.join(format!("h3-key-{}.pem", std::process::id())),
    );
    std::fs::write(&cert, ck.cert.pem()).unwrap();
    std::fs::write(&key, ck.key_pair.serialize_pem()).unwrap();
    let (cert, key) = (cert.display().to_string(), key.display().to_string());

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, async {
        let l = Arc::new(bind("127.0.0.1:0", &BindOptions::default()).unwrap());
        let addr = l.local_addr();
        let port = addr.rsplit(':').next().unwrap().to_string();
        let opts = ServerOptions {
            http3: true,
            tls: Some(tls::server_config(&cert, &key, &[]).unwrap()),
            ..Default::default()
        };
        let server = tokio::task::spawn_local(serve(l.local().unwrap(), opts, handler()));

        let url = url::Url::parse(&format!("https://localhost:{port}/h3?x=1")).unwrap();
        let got = tokio::time::timeout(
            Duration::from_secs(10),
            h3::fetch(
                "POST",
                &url,
                &[("content-type".into(), "text/plain".into())],
                b"habari".to_vec(),
                Some(&cert),
                1 << 20,
            ),
        )
        .await
        .expect("answers in time")
        .expect("fetches");
        assert_eq!(got.status, 201);
        assert_eq!(String::from_utf8(got.body).unwrap(), "POST /h3 habari");
        assert!(got
            .headers
            .iter()
            .any(|(k, v)| k == "x-toleo" && v == "HTTP/3"));

        // The limit applies to the answer.
        let small = h3::fetch("GET", &url, &[], vec![], Some(&cert), 4).await;
        assert!(small.unwrap_err().contains("kikomo"));

        l.stop();
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("serve ends once stopped")
            .unwrap();
    });
    let _ = std::fs::remove_file(&cert);
    let _ = std::fs::remove_file(&key);
}
