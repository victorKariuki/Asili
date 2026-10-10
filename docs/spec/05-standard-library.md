A `sawia kazi` (async function) runs as a **task** on the calling thread. Calling it starts the
task and gives an `Ahadi<T>` (a promise of its return value `T`); `subiri a` (await) waits for
that value, re-raising a `paparika` from the task. While a task waits — `lala`, a `njia`
`.tuma`/`.pokea`, `subiri`, network I/O — the thread's other tasks run, so many waits overlap
on one thread. Tasks start when first waited on (or when the starting call returns), and a
top-level call returns only once every task it started has finished.

```asili
leta sambamba
leta majira

sawia kazi baada(sekunde: Namba, jina: Neno) -> Neno {
    lala(sekunde)
    rejesha jina
}

kazi kuu() -> Tupu {
    weka a = baada(0.3, "a")
    weka b = baada(0.3, "b")
    chapisha(subiri a + subiri b)   # "ab", after about 0.3 s, not 0.6 s
}
```

| `sambamba` | |
|---|---|
| `anzisha(kazi_jina, ...hoja)` | Start any function as a task: its `Ahadi`. |
| `subiri_zote(orodha)` | Every task's value, in order. |
| `subiri_yoyote(orodha)` | The first to finish: `(nafasi, thamani)`. |
| `muda_kikomo(ahadi, sekunde)` | The value, or `Hamna` if time runs out first (the task goes on). |
| `ghairi(ahadi)` / `a.ghairi()` | Cancel: the task's current or next wait returns an error. |
| `a.imekwisha()` | Whether the task has finished. |

`kazi kuu` cannot be `sawia` (`SEM106`); `subiri` needs an `Ahadi` (`SEM107`). Tasks are
single-threaded: an `Ahadi` cannot be sent to another thread; use `tenda` and `njia` for
parallelism. In the browser playground a task runs to completion when started.

---

# 5. Standard Library (Msingi / Mfumo)

Previous: [Type System](04-type-system.md) | [Overview](../SPECIFICATION.md) | Next: [Tooling and Ecosystem](06-tooling-and-ecosystem.md)

---

## Principles

The Asili Standard Library is governed by the EDP. Its purpose is to provide a robust **Substrate** and minimize **Friction** between thought and execution.

- **Urahisi (Simplicity):** Core functions use intuitive Swahili terminology.
- **Nguvu (Performance):** Abstractions are zero-cost where possible; the library should not impose unnecessary cost on systems-level code.
- **Umoja (Consistency):** Naming conventions are uniform across all modules.
- **Explicit failure:** Functions that can fail return **Tokeo\<T, E\>**; no silent failure.
- **Portability:** The **Msingi (Core)** module is **no_std** compatible for bare-metal/embedded use.

---

## Moduli ya Msingi (Core — no_std)

Default-imported. No OS dependency. The Phase I interpreter provides selected stdlib as **builtin modules** (e.g. `hisabati`, `mfumo`) resolved without disk I/O; `leta hisabati` and `leta mfumo` work without corresponding `.asi` files. The same modules are also exposed as an **in-language surface** in **lib/std/** (`.asi`/`.as`), so the stdlib is both built-in and “written in the language.” Implementation may be split by module (hisabati, mfumo, neno, …) for scalability without changing the specified API.

### Hisabati (Math)

- **Infallible:** `jumla(a, b)`, `tofauti(a, b)`, `zao(a, b)` — basic arithmetic (`Namba` follows IEEE 754; fixed-width `Biti*`/`uBiti*` operators wrap deterministically).
- **Fallible:** `gawio(a, b)`, `kipeo(n, p)`, `mizizi(n)` — return **Tokeo\<Namba, Kosa\>** (e.g. divide by zero, negative sqrt).
- **Infallible helpers:** `duara(n)` (rounding), `absolute(n)` (absolute value) return `Namba`.
- **Special numeric constants:** `Ukomo` (positive infinity) and `Siyo_Namba` (NaN) are exposed for numeric logic on `Namba`.
- **Random:** `nasibu()`, `nasibu_kamili(a, b)` (bounds included), `changanya(orodha)`, `chagua_nasibu(orodha) -> Chaguo<T>`; `nasibu_mbegu(n)` seeds the thread's generator so the sequence repeats.

### Neno (String)

- `neno.urefu()` — length in **grapheme clusters** (Lugha-Mama).
- `neno.biti_ngapi()` — length in **bytes** (Nguvu).
- `neno.unganisha()`, `neno.kata()`, `neno.tafuta()` — positions count characters.
- `safisha`/`safisha_mwanzo`/`safisha_mwisho`, `jaza_kushoto`/`jaza_kulia`, `jaza` (`{}` templates), `herufi`, `mistari`, `geuza`, `misimbo`, `kwa_namba -> Tokeo<Namba, Neno>`.

### Orodha (List/Array)

- `orodha.ongeza()`, `orodha.ondoa()`, `orodha.kila_mmoja()`.
- `panga`, `panga_kwa`, `geuza`, `kata`, `ina`, `tafuta`, `kubwa`, `ndogo`, `jumla`, `kipekee`, `kwanza`, `mwisho`, `tupu`, `ongeza_zote`, `futa_zote` (full contracts: [08-njia-za-aina.md](../language/08-njia-za-aina.md)).

### Kasha_GC\<T\> (opt-in managed memory)

Unlike the other builtin modules, `kasha_gc` is **not** ambient: `leta kasha_gc` is required
(managed memory is opt-in; `core/parser/src/builtins.rs::OPT_IN_MODULES`). A minimal, deliberately small reference-counted
wrapper (`Rc<RefCell<Value>>`), the concrete realization of the "managed/GC modules" concept in
[07-execution-and-roadmap.md](07-execution-and-roadmap.md). Sharing is explicit via
`.shirikisha()` (like Rust's `Rc::clone`) — a plain `weka b = a` still moves, so wrapping in
`Kasha_GC<T>` never silently changes the language's default move semantics. Not a
tracing/cycle-collecting GC: a self-referential `Kasha_GC<T>` leaks, by design. The escape hatch:
`kasha_gc_dhaifu(kgc) -> Kasha_GC_Dhaifu<T>` downgrades to a weak reference that does not itself
count toward the strong refcount; `.imarisha() -> Chaguo<Kasha_GC<T>>` upgrades back (`Hamna` if
every strong handle has already dropped). Holding the weak reference in one side of a cycle
(e.g. a child's back-pointer to its parent) lets the strong count still reach zero.

---

## Moduli ya Mfumo (System)

Builtin exports are ambient; `leta mfumo` remains optional documentation. OS-dependent.

### Ingizo/Tokeo (I/O)

- `chapisha(ujumbe)` — print to stdout.
- `omba(swali)` — read from stdin with prompt.
- `soma_faili(njia)`, `andika_faili(njia, data)`.

### Mazingira (Environment)

- `mfumo.majira()`, `mfumo.vigezo()`, `mfumo.pata_env(jina)`, `mfumo.toka(kodi)`.
- `weka_env(jina, thamani)`; `endesha(amri, hoja) -> Tokeo<Neno, Neno>` (stdout, or a `Kosa` with exit code and stderr).

### Majira and Faili

- `majira`: `kwa_iso`, `kutoka_iso -> Tokeo<Wakati, Neno>`, `umbiza_eneo(w, dakika)`, `kutoka_tarehe`, `tarehe` (calendar fields), `kipima_muda` (monotonic) — proleptic Gregorian, UTC.
- `faili`: `orodha_saraka`, `unda_saraka`, `futa_saraka`, `badili_jina`, `nakili`, `ni_saraka`, `ni_faili`, `njia_unganisha`, `njia_mzazi`, `njia_jina`, `njia_kiendelezi`, `njia_kamili`.

### JSON

- `kwa_json(thamani) -> Tokeo<Neno, Neno>` — serializes any JSON-representable `Value` to a JSON
  string. Rejects (`Kosa`) resource handles (`Kasha_GC<T>`, `Faili`, `Mkondo`,
  `Kumbukumbu<T>`) and concurrency primitives (`NjiaTx`/`NjiaRx`/`Fungo`, and their bounded
  variants) — none have a JSON representation — and structures nested past a fixed depth cap.
  A whole `Namba` within ±2^53 is written without a fraction (`3`, not `3.0`).
  `NambaKuu`/`NambaSahihi`/`Anuani` encode as JSON strings, not numbers (arbitrary precision and
  raw memory addresses don't round-trip through/belong as a JSON number). See
  [json-codec-design.md](../design/json-codec-design.md) for the full per-variant mapping.
- `kutoka_json(neno) -> Tokeo<Kamusi<Neno, Unknown>, Neno>` — parses a JSON string. A JSON object
  decodes to `Kamusi<Neno, _>`; other JSON shapes (array, scalar) still decode correctly at
  runtime, but the declared static return type reflects the common "parse a JSON object" case.

### HTTP (client)

One function, `http_ombi(njia, anwani, chaguo?: ChaguoHttp) -> Tokeo<JibuHttp, Neno>`, sends any
method (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`, `TRACE`, or a non-standard
token such as `PROPFIND`) to an `http://` or `https://` URL. It returns the whole response,
whatever its status; a `Kosa` is a failed connection, TLS handshake, timeout, read or invalid
option — or a non-2xx status when `kosa_hali` is set. `ChaguoHttp`, `JibuHttp` and `OmbiHttp`
are builtin `umbo`s: a program uses them without declaring them, and every `ChaguoHttp` field
may be left out of a literal.

| `ChaguoHttp` field | Type | Meaning |
|---|---|---|
| `vichwa` | `Kamusi<Neno, Neno>` | Request headers |
| `hoja` | `Kamusi<Neno, Neno>` | Appended to the URL's query, percent-encoded |
| `mwili` | `Neno` | Text body (`text/plain; charset=utf-8`) |
| `mwili_base64` | `Neno` | Binary body, given as base64 (`application/octet-stream`) |
| `json` | any | A value sent as JSON (`application/json`), encoded as `kwa_json` does |
| `fomu` | `Kamusi<Neno, Neno>` | `application/x-www-form-urlencoded` form |
| `fomu_sehemu` / `fomu_faili` | `Kamusi<Neno, Neno>` | `multipart/form-data`: text parts / file parts (part name → path) |
| `faili` | `Neno` | Send this file as the body, streamed |
| `aina` | `Neno` | `Content-Type` instead of the body's default |
| `mtumiaji` / `nenosiri` | `Neno` | Basic authentication |
| `tokeni` | `Neno` | Bearer authentication |
| `muda` | `Namba` | Seconds for the whole request (default 60; `0`: none) |
| `muda_kuunganisha` | `Namba` | Seconds to connect |
| `elekezo` | `Namba` | Redirects followed (default 10; `0`: the 3xx response is returned) |
| `jaribu_tena` | `Namba` | Retries (up to 10) of an idempotent method after a failed connection or a 429/502/503/504, waiting 0.5 s, 1 s, 2 s, … or the server's `Retry-After` (at most 30 s) |
| `wakala` | `Neno` | Proxy: `http://host:port`, `socks5://…` (with `user:password@`); `""` ignores `HTTPS_PROXY`/`HTTP_PROXY` |
| `familia_ip` | `Namba` | `4` or `6`: IPv4 or IPv6 only |
| `cheti_ca` | `Neno` | PEM file of the only certificate authorities trusted (instead of the Mozilla roots) |
| `cheti` / `ufunguo` | `Neno` | Client certificate and private key (PEM), for mutual TLS |
| `vidakuzi` | `Ukweli` | Use the program's one cookie jar: store `Set-Cookie`, send `Cookie` |
| `kikomo` | `Namba` | Most bytes of response body read (default 64 MiB) |
| `hifadhi` | `Neno` | Stream the body to this file (written whole or not at all); `mwili` is then empty |
| `jibu_base64` | `Ukweli` | Return the body as base64 (binary responses) |
| `kosa_hali` | `Ukweli` | A non-2xx status is a `Kosa` with the status and the start of the body |

At most one body option may be given. `JibuHttp` holds `hali` (status), `vichwa` (lowercase
names; a repeated header joined with `", "`), `mwili`, `sababu` (reason phrase), `anwani` (the
final URL), `toleo` (`"HTTP/1.1"`), `vichwa_vyote` (every header in order as `Jozi`, repeats kept
— `set-cookie`) and `muda` (seconds taken).

Behaviour: HTTPS uses rustls with the Mozilla roots, and certificate checking cannot be turned
off. A redirect from HTTPS to plain HTTP is refused; a redirect to another origin drops
`Authorization`, `Proxy-Authorization` and `Cookie`; `303` (and `301`/`302` after a `POST`)
continues as a `GET` without the body, `307`/`308` keep method and body. gzip and brotli responses
are decoded; a body is decoded in its declared charset (UTF-8 otherwise). Requests with the same
transport settings (proxy, roots, client certificate, IP family) share a keep-alive connection
pool across the program and its threads. Not available in the browser build.

### Ruwaza (regular expressions)

`ruwaza_inalingana`, `ruwaza_tafuta`, `ruwaza_zote`, `ruwaza_vikundi`, `ruwaza_badilisha`,
`ruwaza_gawanya` — each `(ruwaza, maandishi, ...)` returning `Tokeo<_, Neno>`, a `Kosa` for an
invalid pattern. Matching takes time linear in the text (no look-around or backreferences).

### Usimbaji (encodings and hashes)

`base64_simba`, `base64_fumbua -> Tokeo<Neno, Neno>`, `hex_simba`, `hashi_sha256`,
`hashi_sha512`, `hmac_sha256(ufunguo, ujumbe)` (hex digests of the text's UTF-8 bytes) and
`kitambulisho()` (a random version-4 UUID, from the same generator as `nasibu`).

---

### Resource handles and traits

System resources are represented by explicit handle types that own their underlying OS or hardware resource and release it on drop. Builtin `Faili`/`Mkondo` exports are ambient; `leta faili`/`leta mfumo` remain optional documentation. `Kumbukumbu<T>` is also in scope, like `Orodha`/`Kamusi`.

- **`Faili`** — file handle. `faili_fungua(njia, hali) -> Tokeo<Faili, Neno>` opens a file; `hali` is `"soma"`, `"andika"`, or `"ongeza"`. Methods: `.soma() -> Tokeo<Neno, Neno>`, `.andika(data: Neno) -> Tokeo<Tupu, Neno>`, `.funga() -> Tupu` (idempotent — closing an already-closed handle is a safe no-op).
- **`Mkondo`** — a connection: TCP, TLS over TCP, or a Unix socket. `mkondo_unganisha(anwani, chaguo?: ChaguoMkondo) -> Tokeo<Mkondo, Neno>` connects to `"mwenyeji:mlango"`, `"[::1]:mlango"` or `"unix:/njia"`; `ChaguoMkondo` has `tls`, `jina_seva`, `muda` (connect timeout), `cheti_ca`, `cheti`/`ufunguo` (mutual TLS) and `familia_ip`. Methods: `.soma()` (to the end), `.soma_baiti(n)` / `.soma_bailisi(n)` (one read of up to `n` bytes, as `Baiti` / text), `.soma_kamili(n)` (exactly `n` bytes), `.soma_mstari() -> Tokeo<Neno?, Neno>` (a line without `\n`/`\r\n`; `Hamna` at the end), `.andika(Neno | Baiti)`, `.funga_kuandika()` (half-close), `.funga()`, `.anwani_mbali()` / `.anwani_yangu()`, `.weka_muda(sekunde)` (time limit of each read and write; 0: none). Inside a `sawia` task, a wait on a `Mkondo` lets the thread's other tasks run. `tafuta_anwani(jina) -> Tokeo<Orodha<Neno>, Neno>` resolves a host name.
- **`MkondoSikilizaji`** — a listening socket. `mkondo_sikiliza(anwani, chaguo?: ChaguoSikiliza) -> Tokeo<MkondoSikilizaji, Neno>` listens (port 0: any free port; `"unix:/njia"`; `ChaguoSikiliza` has `tumia_tena` (`SO_REUSEPORT`) and `foleni` (backlog)). Methods: `.kubali() -> Tokeo<Mkondo, Neno>` (the next connection), `.anwani()` (with the real port), `.simama()` (stop: every pending and later accept ends, on every thread). A listener is a shared handle: copies (and `tenda` arguments) are the same listener. It can also be passed to one of two worker-pool entry points, both running `idadi_ya_nyuzi` worker threads until the listener is stopped, with `tls: Chaguo<TlsUsanidi>` optional in both (omit it, or pass `Chaguo::Hamna`, for a plaintext listener):
  - **`mkondo_tumikia(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls) -> Tokeo<Tupu, Neno>`** — raw bytes. Each worker accepts a connection (TLS handshake first if `tls` was given; a handshake failure drops just that connection, not the worker; each read and write is limited to 30 seconds) and calls `kazi_jina(mkondo: Mkondo) -> Tupu`, which owns that connection's whole lifecycle via `.soma()`/`.andika()`/`.funga()`/`.soma_bailisi(kikomo)` — plaintext or TLS, transparently.
  - **`mkondo_tumikia_http(sikilizaji, kazi_jina, idadi_ya_nyuzi, tls) -> Tokeo<Tupu, Neno>`** — HTTP/1.1 framing. Each worker parses one request per cycle (`Content-Length`-bodied; `Transfer-Encoding: chunked` returns `501`, not supported), calls `kazi_jina(ombi: OmbiHttp) -> JibuHttp`, writes a real HTTP/1.1 response, and — unlike the raw-bytes path — loops to parse the *next* request on the same connection under HTTP/1.1 keep-alive (default unless `Connection: close`), closing only on `Connection: close`, a parse error (`400`), or the peer disconnecting. `OmbiHttp { njia: Neno, anwani: Neno, vichwa: Kamusi<Neno, Neno>, mwili: Neno }` / `JibuHttp { hali: Namba, vichwa: Kamusi<Neno, Neno>, mwili: Neno }` — an Asili program using this needs its own matching `umbo` declarations for the analyzer to type-check `ombi`/the returned `JibuHttp`, even though neither struct's *name* is checked at runtime. No pipelining, no `Expect: 100-continue`, no HTTP/2.

  Every accepted connection gets a fixed read/write timeout, and the pool size is itself the concurrent-connection cap — see [faili-mkondo-design.md](../design/faili-mkondo-design.md), [http-server-design.md](../design/http-server-design.md), [tls-design.md](../design/tls-design.md), and [http-framing-design.md](../design/http-framing-design.md).
- **`TlsUsanidi`** — a loaded TLS server certificate/key pair. `tls_sanidi(cheti_njia, ufunguo_njia) -> Tokeo<TlsUsanidi, Neno>` loads a PEM certificate chain and private key from disk; every failure (missing file, malformed PEM, a cert/key that don't match) returns `Tokeo(Kosa(...))`, never panics. Pass the result (wrapped in `Chaguo::Kuna(...)`) as `mkondo_tumikia`'s 4th argument to serve TLS instead of plaintext.
- **`Kumbukumbu<T>`** — heap-allocated box owning a value of type `T`, no OS resource. `kumbukumbu_unda(thamani) -> Kumbukumbu<T>` constructs; `.pata() -> T` reads a clone of the boxed value. No in-place mutation method (`.weka()`) — a method call receives a clone of its receiver, so mutating that clone's box would not be visible through the original binding; reassign the whole `Kumbukumbu` instead (`weka k = kumbukumbu_unda(thamani_mpya)`). This is a real difference from `Kasha_GC<T>`, which shares its allocation and supports true in-place mutation through any live handle.

`Faili`/`Mkondo` implement deterministic cleanup: the underlying OS handle is closed when the owner goes out of scope, whether by explicit `tupa`, an explicit `.funga()` call, or ordinary block/function exit with no explicit cleanup at all — implemented as a real destructor on the handle's wrapper type (not a hook triggered only by explicit `tupa`), so it fires on every code path uniformly. See [faili-mkondo-design.md](../design/faili-mkondo-design.md) for the mechanism.

Common behaviours are expressed via Sifa (traits), for example:

- `Inasomeka` — any type that can be read from (e.g. `Faili`, `Mkondo`).
- `Inandikika` — any type that can be written to.

These traits allow generic I/O functions to work over multiple resource types while preserving ownership and borrowing rules. `Sifa` now carry real method signatures and a completeness check (see [faili-mkondo-design.md](../design/faili-mkondo-design.md)); `Inasomeka`/`Inandikika` are seeded built-in traits, and `Faili`/`Mkondo` satisfy them **by fiat**, not via a checked `impl` block — `Faili`/`Mkondo` are builtin `ValueType`s whose methods are hardcoded Rust dispatch, a different mechanism from user-declared `umbo`/`shughuli ya`, and the completeness checker only inspects real `module.impls` entries (which can only target user-declared structs/enums today). No generic-over-trait function parameters exist yet either (e.g. a function taking "any `Inasomeka`" polymorphically) — that needs a trait-object `Value` representation this interpreter doesn't have.

---

## Moduli ya Takwimu na Safu (Data)

High-performance primitives for multidimensional data, statistics, and large-scale processing.

### Tensors and linear algebra

- `safu.umbo(dim)`, `safu.zao_dot(a, b)`, `safu.geuza()`, `safu.kawaida()`.

### Transformation and calculus

- `safu.gradienti(kazi)`, `safu.takwimu(aina)`, `safu.badili(asili, lengo)`.

### Data loading

- `pakia.csv(njia)`, `pakia.jozi(njia)`, `pakia.gawanya(data, uwiano)`.

---

## Hardware / Nguvu (inside `wazi`)

Available inside **wazi** blocks for hardware-level control.

- `vuta_vifaani(data)` — offload to accelerators (GPU/NPU/DSP).
- `anwani_ya(kitu)` — memory address (pointer).
- `tenga_kumbukumbu(saizi)` — manual heap allocation.
- `piga_pini(namba, hali)` — GPIO for sensor data.

Typed hardware handles may be provided to model hardware state safely:

- `Pini` — GPIO pin configured as input or output.
- `Bafa<T>` — contiguous buffer with alignment guarantees suitable for accelerators (GPU/NPU).

---

## Error model

- **Success:** `SAWA(thamani)`.
- **Failure:** `KOSA(maelezo)` — Mwalimu-friendly message; type is **Tokeo\<T, E\>** with user-definable `E`.

Panic (`paparika`) is reserved for unrecoverable critical conditions (for example, internal invariants or undefined behaviour detected inside `wazi` blocks). Recoverable errors in the standard library must be expressed via `Tokeo\<T, E\>` and resolved using `linganisha` or `jaribu`, not via panic.

---

## Concurrency primitives (tenda, njia, fungo)

Builtin exports are ambient; `leta sambamba` remains optional documentation. **1:1 OS-thread model** (one `tenda` spawns one real OS thread via
`std::thread`, not a green-thread scheduler — see [Resolved Decisions](08-resolved-decisions.md)
and [concurrency-design.md](../design/concurrency-design.md) for why).

- **`tenda(kazi_jina, hoja...) -> Tokeo<Namba, Neno>`** — spawns the named module-level `kazi` on
  a new thread, returning a handle id. Arguments must be `Send`-safe: `Kasha_GC<T>`/`Faili`/
  `Mkondo` (or anything containing one) are rejected with `Kosa`, not silently allowed or
  undefined behavior. A spawned function's own return value does not come back through
  `subiri_tenda` — communicate results via `njia`.
- **`subiri_tenda(id) -> Tokeo<Tupu, Neno>`** — joins the thread, reporting whether it finished
  cleanly or panicked.
- **njia (Channel):** `njia() -> Jozi<NjiaTx<T>, NjiaRx<T>>` creates an unbounded channel
  (infallible — a channel can't fail to construct). `tx.tuma(v) -> Tokeo<Tupu, Neno>` sends;
  `rx.pokea() -> Tokeo<T, Neno>` receives (blocking), `Kosa` once every sender is dropped.
  `njia_na_kikomo(kikomo: Namba) -> Jozi<NjiaTxBounded<T>, NjiaRxBounded<T>>` creates a bounded
  channel instead — same `.tuma()`/`.pokea()` contract, except `.tuma()` blocks once `kikomo`
  unread items are already buffered, rather than growing memory without limit the way the
  unbounded `njia()` does under a producer faster than its consumer.
- **fungo (Mutex/Lock):** `fungo(thamani) -> Tokeo<Fungo<T>, Neno>` wraps a `Send`-safe value.
  `f.pata() -> T` / `f.weka(v) -> Tupu` lock, act, and unlock atomically in one call — the usual
  way to use a `Fungo`. `f.funga() -> Tupu` / `f.fungua() -> Tupu` are separate, explicit
  lock/unlock for holding the lock across several operations; calling `.fungua()` without a
  matching prior `.funga()` is a programming error (reported as a panic).

Full concurrency model: **1:1 scheduling** is the resolved decision (see
[Resolved Decisions](08-resolved-decisions.md)).

---

## Async (sawia, subiri)

**sawia** (async) and **subiri** (await) — Future surface for non-blocking execution. Full semantics are deferred to [Execution and Roadmap](07-execution-and-roadmap.md) and 08 when the async phase is defined; they require an executor/runtime.

---

Previous: [Type System](04-type-system.md) | [Overview](../SPECIFICATION.md) | Next: [Tooling and Ecosystem](06-tooling-and-ecosystem.md)
