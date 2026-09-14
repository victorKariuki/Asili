HTTP/1.1 request/response framing example: mkondo_tumikia_http serving a small routed responder.
See docs/design/http-framing-design.md and docs/design/http-server-design.md.

Run with `pata jenga --tenda` from this directory, then in another terminal:

    curl http://127.0.0.1:8080/
    curl -X POST -d "hello" http://127.0.0.1:8080/echo
    curl http://127.0.0.1:8080/anything-else   # 404

The server never returns (mkondo_tumikia_http blocks forever) — stop it with Ctrl-C.

Contrast with examples/mkondo_server/, which uses the lower-level raw-bytes mkondo_tumikia:
kazi_jina there gets a live Mkondo handle and owns the whole connection (one request per
connection, no HTTP semantics). Here, kazi_jina gets a parsed OmbiHttp and returns a JibuHttp —
the framing layer owns request parsing, response writing, and HTTP/1.1 keep-alive.
