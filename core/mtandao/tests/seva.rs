use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use asili_mtandao::http::seva::{serve, Handler, Jibu, JibuMwili, Ombi, ServerOptions};
use asili_mtandao::{bind, BindOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn handler() -> Handler {
    Rc::new(|o: Ombi| {
        Box::pin(async move {
            match o.path.as_str() {
                "/mkondo" => {
                    let chunks = futures_stream(vec![b"moja ".to_vec(), b"mbili".to_vec()]);
                    Jibu {
                        status: 200,
                        reason: None,
                        headers: vec![],
                        body: JibuMwili::Stream(chunks),
                    }
                }
                "/kubwa" => Jibu {
                    status: 200,
                    reason: None,
                    headers: vec![("content-type".into(), "text/plain".into())],
                    body: JibuMwili::Full(b"a".repeat(5000)),
                },
                _ => Jibu {
                    status: 201,
                    reason: Some("Imeundwa".into()),
                    headers: vec![("x-njia".into(), o.method.clone())],
                    body: JibuMwili::Full(
                        format!(
                            "{} {} {:?} {} {}",
                            o.path,
                            o.target,
                            o.query,
                            o.version,
                            String::from_utf8_lossy(&o.body)
                        )
                        .into_bytes(),
                    ),
                },
            }
        })
    })
}

fn futures_stream(items: Vec<Vec<u8>>) -> asili_mtandao::http::seva::Chunks {
    struct It(std::vec::IntoIter<Vec<u8>>);
    impl futures_core::Stream for It {
        type Item = Vec<u8>;
        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Vec<u8>>> {
            std::task::Poll::Ready(self.0.next())
        }
    }
    Box::pin(It(items.into_iter()))
}

/// One HTTP/1.1 exchange on a fresh connection: the whole response as text.
async fn exchange(addr: &str, request: &str) -> String {
    let mut s = TcpStream::connect(addr).await.unwrap();
    s.write_all(request.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    s.read_to_end(&mut out).await.unwrap();
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn http1_requests_streams_compression_limits_and_stop() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, async {
        let l = Arc::new(bind("127.0.0.1:0", &BindOptions::default()).unwrap());
        let addr = l.local_addr();
        let opts = ServerOptions {
            body_limit: 64,
            ..Default::default()
        };
        let server = tokio::task::spawn_local(serve(l.local().unwrap(), opts, handler()));

        let r = exchange(
            &addr,
            "POST /a%20b?x=1&y=n%C3%A9 HTTP/1.1\r\nhost: h\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhabari",
        )
        .await;
        assert!(r.starts_with("HTTP/1.1 201 Imeundwa\r\n"), "{r}");
        assert!(r.contains("x-njia: POST"), "{r}");
        assert!(r.contains("date: "), "{r}");
        assert!(
            r.ends_with(r#"/a b /a%20b?x=1&y=n%C3%A9 [("x", "1"), ("y", "né")] HTTP/1.1 habar"#),
            "{r}"
        );

        let r = exchange(&addr, "HEAD / HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n").await;
        assert!(r.contains("content-length: 16"), "{r}");
        assert!(r.ends_with("\r\n\r\n"), "HEAD has no body: {r}");

        let r = exchange(&addr, "GET /mkondo HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n").await;
        assert!(r.contains("transfer-encoding: chunked"), "{r}");
        assert!(r.contains("moja ") && r.contains("mbili"), "{r}");

        let r = exchange(
            &addr,
            "GET /kubwa HTTP/1.1\r\nhost: h\r\naccept-encoding: gzip\r\nconnection: close\r\n\r\n",
        )
        .await;
        assert!(r.contains("content-encoding: gzip") && r.len() < 1000, "{r}");

        let big = "x".repeat(100);
        let r = exchange(
            &addr,
            &format!("POST / HTTP/1.1\r\nhost: h\r\ncontent-length: 100\r\nconnection: close\r\n\r\n{big}"),
        )
        .await;
        assert!(r.starts_with("HTTP/1.1 413"), "{r}");

        // Two pipelined requests on one connection.
        let r = exchange(
            &addr,
            "GET /1 HTTP/1.1\r\nhost: h\r\n\r\nGET /2 HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n",
        )
        .await;
        assert_eq!(r.matches("HTTP/1.1 201").count(), 2, "{r}");

        l.stop();
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("serve ends once stopped")
            .unwrap();
    });
}

#[test]
fn http2_with_prior_knowledge() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, async {
        let l = Arc::new(bind("127.0.0.1:0", &BindOptions::default()).unwrap());
        let addr = l.local_addr();
        tokio::task::spawn_local(serve(
            l.local().unwrap(),
            ServerOptions::default(),
            handler(),
        ));

        let io = hyper_util::rt::TokioIo::new(TcpStream::connect(&addr).await.unwrap());
        let (mut send, conn) =
            hyper::client::conn::http2::handshake(hyper_util::rt::TokioExecutor::new(), io)
                .await
                .unwrap();
        tokio::task::spawn_local(conn);
        let req = hyper::Request::get(format!("http://{addr}/h2?a=b"))
            .body(http_body_util::Empty::<bytes::Bytes>::new())
            .unwrap();
        let resp = send.send_request(req).await.unwrap();
        assert_eq!(resp.status(), 201);
        assert_eq!(resp.version(), hyper::Version::HTTP_2);
        use http_body_util::BodyExt;
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], br#"/h2 /h2?a=b [("a", "b")] HTTP/2 "#);
        l.stop();
    });
}
