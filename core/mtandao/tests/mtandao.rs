use std::sync::Arc;
use std::time::Duration;

use asili_mtandao::{bind, connect, resolve, split_host_port, tls, BindOptions, ConnectOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn temp(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("asili-mtandao-{}-{name}", std::process::id()))
        .display()
        .to_string()
}

/// Accept one connection on `listener` and echo it back until EOF.
async fn echo_once(
    listener: Arc<asili_mtandao::Listener>,
    server_tls: Option<Arc<rustls::ServerConfig>>,
) {
    let local = listener.local().unwrap();
    let (mut s, _) = local.accept().await.unwrap().unwrap();
    if let Some(cfg) = server_tls {
        s = tls::accept(cfg, s).await.unwrap();
    }
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).await.unwrap();
    s.write_all(&buf).await.unwrap();
    s.shutdown().await.unwrap();
}

async fn round_trip(addr: &str, opts: &ConnectOptions) -> String {
    let mut c = connect(addr, opts).await.unwrap();
    c.write_all(b"habari").await.unwrap();
    c.shutdown().await.unwrap();
    let mut out = String::new();
    c.read_to_string(&mut out).await.unwrap();
    out
}

#[tokio::test(flavor = "current_thread")]
async fn tcp_and_unix_echo() {
    let l = Arc::new(bind("127.0.0.1:0", &BindOptions::default()).unwrap());
    let addr = l.local_addr();
    assert!(
        addr.starts_with("127.0.0.1:") && !addr.ends_with(":0"),
        "{addr}"
    );
    let server = tokio::spawn(echo_once(l.clone(), None));
    assert_eq!(
        round_trip(&addr, &ConnectOptions::default()).await,
        "habari"
    );
    server.await.unwrap();

    let path = temp("echo.sock");
    let l = Arc::new(bind(&format!("unix:{path}"), &BindOptions::default()).unwrap());
    let server = tokio::spawn(echo_once(l.clone(), None));
    assert_eq!(
        round_trip(&format!("unix:{path}"), &ConnectOptions::default()).await,
        "habari"
    );
    server.await.unwrap();
    drop(l);
    assert!(!std::path::Path::new(&path).exists(), "socket file removed");
}

#[tokio::test(flavor = "current_thread")]
async fn tls_with_a_private_root() {
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let (cert, key) = (temp("cert.pem"), temp("key.pem"));
    std::fs::write(&cert, ck.cert.pem()).unwrap();
    std::fs::write(&key, ck.key_pair.serialize_pem()).unwrap();
    let server_cfg = tls::server_config(&cert, &key, &[b"h2", b"http/1.1"]).unwrap();

    let l = Arc::new(bind("127.0.0.1:0", &BindOptions::default()).unwrap());
    let addr = l.local_addr();
    let server = tokio::spawn(echo_once(l.clone(), Some(server_cfg)));
    let opts = ConnectOptions {
        tls: true,
        server_name: Some("localhost".into()),
        ca_file: Some(cert.clone()),
        ..Default::default()
    };
    assert_eq!(round_trip(&addr, &opts).await, "habari");
    server.await.unwrap();

    // The Mozilla roots do not trust a self-signed certificate.
    let l = Arc::new(bind("127.0.0.1:0", &BindOptions::default()).unwrap());
    let addr = l.local_addr();
    let cfg = tls::server_config(&cert, &key, &[]).unwrap();
    tokio::spawn(async move {
        let local = l.local().unwrap();
        let (s, _) = local.accept().await.unwrap().unwrap();
        let _ = tls::accept(cfg, s).await;
    });
    let opts = ConnectOptions {
        tls: true,
        server_name: Some("localhost".into()),
        ..Default::default()
    };
    let err = connect(&addr, &opts).await.err().unwrap();
    assert!(err.contains("TLS"), "{err}");

    let err = tls::server_config(&temp("hakuna.pem"), &key, &[])
        .err()
        .unwrap();
    assert!(err.contains("hakuna.pem"), "{err}");
}

#[tokio::test(flavor = "current_thread")]
async fn stop_ends_pending_accepts() {
    let l = Arc::new(bind("127.0.0.1:0", &BindOptions::default()).unwrap());
    let local = l.local().unwrap();
    let stopper = l.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        stopper.stop();
    });
    let got = tokio::time::timeout(Duration::from_secs(5), local.accept())
        .await
        .expect("accept ends when stopped")
        .unwrap();
    assert!(got.is_none());
    assert!(local.accept().await.unwrap().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn names_addresses_and_timeouts() {
    let ips = resolve("localhost").await.unwrap();
    assert!(
        ips.iter().any(|ip| ip == "127.0.0.1" || ip == "::1"),
        "{ips:?}"
    );
    assert_eq!(
        split_host_port("[::1]:80").unwrap(),
        ("::1".to_string(), 80)
    );
    assert_eq!(
        split_host_port("mfano.com:443").unwrap(),
        ("mfano.com".to_string(), 443)
    );
    assert!(split_host_port("hakuna_mlango").is_err());

    // Nothing listens on a port that was just freed: refused.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let err = connect(&format!("127.0.0.1:{port}"), &ConnectOptions::default())
        .await
        .err()
        .unwrap();
    assert!(err.starts_with("kuunganisha"), "{err}");
}

#[tokio::test(flavor = "current_thread")]
async fn udp_datagrams_and_default_peer() {
    let a = asili_mtandao::udp_bind("127.0.0.1:0").await.unwrap();
    let b = asili_mtandao::udp_bind("127.0.0.1:0").await.unwrap();
    a.send_to(b"jambo", &b.local_addr()).await.unwrap();
    let (data, from) = b.recv_from(1500).await.unwrap();
    assert_eq!((data.as_slice(), from), (&b"jambo"[..], a.local_addr()));
    b.connect(&a.local_addr()).await.unwrap();
    b.send(b"sawa").await.unwrap();
    assert_eq!(
        a.recv_from(2).await.unwrap().0,
        b"sa",
        "a longer datagram is cut"
    );
    b.set_broadcast(true).unwrap();
    assert!(b.join_multicast("si-anwani").is_err());
}
