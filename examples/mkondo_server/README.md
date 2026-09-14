Mkondo listener + worker-pool example: mkondo_sikiliza/mkondo_tumikia serving a plaintext
echo-style responder over a bounded thread pool. See docs/design/http-server-design.md and
docs/design/faili-mkondo-design.md's "Listening sockets and the worker pool" section.

Run with `pata jenga --tenda` from this directory, then in another terminal:

    echo -n "habari" | nc 127.0.0.1 7878

The server never returns (mkondo_tumikia blocks forever) — stop it with Ctrl-C.
