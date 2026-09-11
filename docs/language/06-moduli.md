# Moduli (Modules)

## Kuingiza Moduli (Importing)

```asili
leta matumizi
leta hisabati
```

`leta` at the top of a file imports a standard library module. All its exported functions become available without a namespace prefix.

---

## Moduli za Kawaida (Standard Library Modules)

### matumizi — I/O

```asili
leta matumizi

chapisha("Habari Dunia!")           # print to stdout
onyo("Tahadhari!")                  # print to stderr (warning prefix)
makosa("Kosa kubwa")                # print to stderr (error prefix)
paparika("Kosa la dharura")         # print error and exit immediately

weka ingizo = omba("Jina lako: ")   # read line from stdin
```

### hisabati — Hesabu

```asili
leta hisabati

weka j = jumla(3, 4)          # 7
weka t = tofauti(10, 4)       # 6
weka z = zao(3, 5)            # 15
weka g = jaribu gawio(10, 2)  # 5 (Tokeo)

weka mz = jaribu mizizi(9)    # 3.0 (Tokeo; fails if n < 0)
weka d = duara(7.4)           # 7 (round to nearest — not "square")
weka k = jaribu kipeo(2, 10)  # 1024 (Tokeo)

weka ab = absolute(-5)        # 5
weka s = sakafu(3.7)          # 3
weka juu = dari(3.2)          # 4
weka n = nasibu()             # random float [0,1)
```

**Maadili ya hisabati (constants)** — available after `leta hisabati`:

| Jina       | Thamani                    | Maelezo                      |
|------------|----------------------------|------------------------------|
| `PI`       | 3.14159265358979…          | π                            |
| `E`        | 2.71828182845904…          | Namba ya Euler               |
| `TAU`      | 6.28318530717958…          | 2π                           |
| `PHI`      | 1.61803398874989…          | Uwiano wa Dhahabu (φ)        |
| `LN2`      | 0.693147…                  | ln(2)                        |
| `LN10`     | 2.302585…                  | ln(10)                       |
| `LOG2E`    | 1.442695…                  | log₂(e)                      |
| `LOG10E`   | 0.434294…                  | log₁₀(e)                     |
| `KIPEUO1_2`| 0.707106…                  | 1/√2                         |
| `KIPEUO2`  | 1.414213…                  | √2                           |
| `KIPEUO3`  | 1.732050…                  | √3                           |
| `KIPEUO5`  | 2.236067…                  | √5                           |
| `EPSILON`  | 2.220446e-16               | Tofauti ndogo zaidi ya float |
| `INF`      | ∞                          | Ukomo chanya                 |
| `NAN`      | NaN                        | Siyo Namba                   |
| `Ukomo`    | ∞                          | Ukomo (alias ya INF)         |
| `Siyo_Namba`| NaN                       | Siyo Namba (alias ya NAN)    |

### mfumo — System

```asili
leta mfumo

weka hoja = vigezo()                # Orodha<Neno> of CLI args
weka path = pata_env("PATH")       # Chaguo<Neno>
toka(1)                             # exit with code
sikiliza_ishara(2, shimla)          # register signal handler (e.g. SIGINT = 2)
rejesha_ishara(2)                   # reset signal handler to default
```

**Maadili ya mfumo** — available after `leta mfumo`:

| Jina     | Maelezo                         |
|----------|---------------------------------|
| `TOLEO`  | Toleo la sasa la Asili (Neno)   |
| `JINA_OS`| Jina la mfumo wa uendeshaji (Neno) |

### majira — Time

```asili
leta majira

weka wakati_sasa = sasa()              # Wakati: current timestamp
weka sek = sekunde(wakati_sasa)        # Namba: seconds since epoch
weka w2 = kutoka_sekunde(1700000000)   # Wakati: from epoch seconds
weka umbizwa = umbiza(wakati_sasa)     # formatted string (no format-string argument today —
                                        # umbiza takes exactly 1 arg: a Wakati)
lala(1)                                 # sleep for 1 second (argument is seconds, not ms)

weka sasa_namba = majira()             # Namba: raw seconds since epoch (not Wakati)
```

> `majira()` returns `Namba` (raw float seconds). `sasa()` returns `Wakati` — a plain opaque
> time value with **no methods of its own**; `sekunde(w)` and `umbiza(w)` are free functions
> that take a `Wakati` argument (`w.sekunde()`/`w.umbiza()` do not exist and fail to compile:
> `SEM039: aina 'Wakati' haina njia`). Use `sasa()` + the free functions above when you need
> formatting; use `majira()` for simple elapsed-time arithmetic.
>
> **Known bug:** `umbiza()`'s calendar-date formatting is inaccurate — it uses fixed 30-day
> months and no leap-year handling, so the date portion can be off by more than two weeks
> depending on the time of year (the time-of-day portion is correct). See
> [implementation-status.md](../design/implementation-status.md).

**Maadili ya majira** — available after `leta majira`:

| Jina                | Thamani  | Maelezo                        |
|---------------------|----------|--------------------------------|
| `SEKUNDE_KWA_SIKU`  | 86400    | Idadi ya sekunde kwa siku      |
| `MWANZO_WA_ZAMANI`  | 0        | Epoch ya Unix (1970-01-01)     |

### faili — File System

```asili
leta faili

weka yaliyomo = jaribu soma_faili("data.txt")
jaribu andika_faili("out.txt", "maudhui\n")
weka ipo = vipo("path/to/file")      # Ukweli
weka ukubwa_w = ukubwa("file.txt")   # Namba (bytes)
jaribu futa("temp.txt")
```

**Maadili ya faili** — available after `leta faili`:

| Jina             | Maelezo                                     |
|------------------|---------------------------------------------|
| `NJIA_SEPARATOR` | Kitenganishi cha njia (`/` au `\` kwenye OS) |

### runtime

```asili
leta runtime

chapisha(toleo())          # e.g. "0.1.0"
chapisha(jina_os())        # e.g. "linux"
chapisha(arch())           # e.g. "x86_64"
chapisha(ni_debug() kama Neno)   # kweli / si_kweli
chapisha(ni_wasm() kama Neno)    # kweli / si_kweli
weka env = mazingira()     # Kamusi<Neno,Neno> of all env vars
weka w = muda_wa_kuanza()  # Wakati: seconds since Unix epoch, captured once at process
                            # start (not milliseconds, not relative to process start time)
```

### kiungo — FFI

```asili
leta kiungo

weka handle = saza_kiungo("./libmylib.so")
weka matokeo = wito_kiungo(handle, "my_function", orodha(1, 2))
```

### sambamba — Concurrency

```asili
leta sambamba

weka mwendo = anza_mwendo(hesabu_kubwa)
weka jibu = subiri_mwendo(mwendo)
```

### kasha_gc — Managed Memory (Kasha_GC\<T\>)

Opt-in reference-counted shared wrapper, layered on top of the default ownership model — not a
replacement for it, and not a tracing/cycle-collecting garbage collector (a `Kasha_GC<T>` that
references itself, directly or through others, leaks; there is no cycle detection).

```asili
leta kasha_gc

weka a = kasha_gc_unda(0.0)     # wrap a value: Kasha_GC<Namba>
weka b = a.shirikisha()         # explicit share: b and a point at the same cell (like Rust's Rc::clone)

b.weka(42.0)                    # mutate through b...
chapisha(a.pata() kama Neno)    # ...visible through a too: "42"

chapisha(a.idadi() kama Neno)   # live handle count sharing this cell: "2"
tupa b                          # dropping a handle decrements the count, same as any other `tupa`
chapisha(a.idadi() kama Neno)   # "1"
```

`weka b = a` alone (without `.shirikisha()`) still **moves** `a`, exactly like any other value —
Kasha_GC does not change move-checking. Sharing a cell always goes through `.shirikisha()`, mirroring
how Rust's own `Rc<T>` requires an explicit `.clone()` rather than sharing on plain assignment.

| Method | Effect |
|---|---|
| `kasha_gc_unda(v)` | Wrap `v`, returning a new `Kasha_GC<T>` handle. |
| `.pata()` | Read a snapshot of the current inner value (a copy, not a live view). |
| `.weka(v)` | Replace the inner value; visible through every other live handle to the same cell. |
| `.shirikisha()` | Share this cell: returns a new handle with its own binding, same underlying cell. |
| `.idadi()` | Number of live handles currently sharing this cell. |

`==` on two `Kasha_GC<T>` handles compares **identity** (do they share the same cell?), not
contents — two separately-`kasha_gc_unda`'d handles with equal inner values are not `==`.

---

## Kuunda Moduli za Mradi Wako

Asili projects are single-file at the entry point (`src/kuu.as`). Cross-file modules are currently imported via `leta` only for the standard library. For multi-file projects, use separate project directories and the `ongeza` dependency system.
