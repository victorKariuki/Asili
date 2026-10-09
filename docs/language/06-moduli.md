# Moduli (Modules)

## Kuingiza Moduli (Importing)

Builtin standard-library modules are available without `leta`, except the opt-in `kasha_gc`
(managed memory), which must be imported explicitly. The keyword remains necessary for
project and dependency modules; it may also be used as documentation when showing which builtin
module provides a function.

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
weka r = nasibu_kamili(1, 6)  # a whole number from 1 to 6, both included
nasibu_mbegu(42)              # from here on, the same numbers every run (tests, simulations)
weka c = changanya([1, 2, 3]) # the list in random order
weka x = chagua_nasibu([4, 5])  # Chaguo: a random element, Hamna for an empty list
```

**Maadili ya hisabati (constants)** — available ambiently:

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

**Namba_Kuu / Namba_Sahihi** (usahihi usio na kikomo, pia available ambiently):

```asili
leta hisabati

weka kubwa = jaribu (namba_kuu_kutoka("999999999999999999999999999999999999999999999999"))
weka moja = jaribu (namba_kuu_kutoka("1"))
chapisha((kubwa + moja) kama Neno)   # halisi, si iliyozungushwa na f64

weka a = jaribu (namba_sahihi_kutoka("0.1"))
weka b = jaribu (namba_sahihi_kutoka("0.2"))
chapisha((a + b) kama Neno)          # "0.3" halisi -- kinyume na 0.1 + 0.2 ya Namba ya kawaida
```

Hakuna sintaksia ya kihalisia (literal) — ujenzi ni kupitia `namba_kuu_kutoka`/
`namba_sahihi_kutoka` (`Tokeo<T, Neno>`) pekee, au ubadilishaji usio na hitilafu kutoka `Namba`
(`namba kama Namba_Kuu`). Ubadilishaji wa kurudi (`kama Namba`) unaweza kushindwa —
`Chaguo<Namba>`.

### mfumo — System

```asili
leta mfumo

weka hoja = vigezo()                # Orodha<Neno> of CLI args
weka path = pata_env("PATH")       # Chaguo<Neno>
toka(1)                             # exit with code
weka_env("LUGHA", "sw")             # set an environment variable for this program (and the
                                    # programs it runs)
weka orodha = jaribu endesha("ls", ["-l"])   # run a program: its output if it exits with 0,
                                    # else a Kosa naming the exit code and its stderr
jaribu mlinzi_anza(100)            # watchdog: feed with mlinzi_lisha() within every 100 ms
kikomo_kumbukumbu(8000000)         # memory limit in bytes (runner); see #[hali_salama]
jaribu sikiliza_ishara(2, shimla)   # register signal handler (e.g. SIGINT = 2); returns
                                     # Tokeo<Tupu, Neno> — Kosa on platforms with no signal
                                     # support (non-Unix)
jaribu rejesha_ishara(2)            # reset signal handler to default; same Tokeo contract
```

**Maadili ya mfumo** — available ambiently:

| Jina     | Maelezo                         |
|----------|---------------------------------|
| `TOLEO`  | Toleo la sasa la Asili (Neno)   |
| `JINA_OS`| Jina la mfumo wa uendeshaji (Neno) |

**Kishikizo cha Mkondo** (TCP client stream, available ambiently):

```asili
leta mfumo

weka m = jaribu (mkondo_unganisha("127.0.0.1:8080"))
jaribu (m.andika("GET / HTTP/1.0\r\n\r\n"))
weka jibu = jaribu (m.soma())
m.funga()
```

`mkondo_unganisha(anwani)` inarejesha `Tokeo<Mkondo, Neno>`. Kishikizo hufungwa kiotomatiki
kikitoka nje ya wigo, kama `Faili` — tazama
[faili-mkondo-design.md](../design/faili-mkondo-design.md).

**Kusikiliza na kutumikia** (listening socket + bwawa la nyuzi lililowekewa kikomo):

```asili
leta mfumo

kazi mtumishi(m: Mkondo) -> Tupu {
    weka ombi = jaribu (m.soma())
    jaribu (m.andika("umetuma bytes " + (ombi kama Neno)))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka sikilizaji = jaribu (mkondo_sikiliza("127.0.0.1:7878"))
    jaribu (mkondo_tumikia(sikilizaji, "mtumishi", 4.0))
}
```

`mkondo_sikiliza(anwani)` inarejesha `Tokeo<MkondoSikilizaji, Neno>`. `mkondo_tumikia(sikilizaji,
kazi_jina, idadi_ya_nyuzi, tls)` huanzisha bwawa la nyuzi za muda mrefu (idadi maalum, si nyuzi
moja kwa kila muunganisho) na huzuia mahali ilipoitwa milele. `mkondo_tumikia_http` ni sawa lakini
hufasiri HTTP/1.1 halisi — `kazi_jina(ombi: OmbiHttp) -> JibuHttp` badala ya
`kazi_jina(mkondo: Mkondo) -> Tupu`. TLS (hoja ya nne, `tls: Chaguo<TlsUsanidi>`, hiari) hutumia
`tls_sanidi(cheti_njia, ufunguo_njia)` kupakia cheti/ufunguo. Maelezo kamili:
[http-server-design.md](../design/http-server-design.md),
[tls-design.md](../design/tls-design.md), [http-framing-design.md](../design/http-framing-design.md).

**JSON**:

```asili
leta mfumo

weka jsoni = jaribu (kwa_json(orodha(1.0, 2.0, 3.0)))  # Tokeo<Neno, Neno>
weka thamani = jaribu (kutoka_json(jsoni))  # Tokeo<Kamusi<Neno, Unknown>, Neno>
```

Vishikizo vya rasilimali (`Kasha_GC<T>`, `Faili`, `Mkondo`) na miundo ya sambamba (`NjiaTx`/
`NjiaRx`/`Fungo`) hazina uwakilishi wa JSON — hukataliwa na `Tokeo(Kosa(...))`. Tazama
[json-codec-design.md](../design/json-codec-design.md).

### majira — Time

```asili
leta majira

weka wakati_sasa = sasa()              # Wakati: current timestamp
weka sek = sekunde(wakati_sasa)        # Namba: seconds since epoch
weka w2 = kutoka_sekunde(1700000000)   # Wakati: from epoch seconds
weka umbizwa = umbiza(wakati_sasa)     # "2026-10-09 12:30:00" (UTC)
weka iso = kwa_iso(wakati_sasa)         # "2026-10-09T12:30:00Z" (milliseconds when present)
weka eneo = umbiza_eneo(wakati_sasa, 180)  # "2026-10-09 15:30:00+03:00": UTC+3, in minutes
weka w3 = jaribu kutoka_iso("2026-10-09T15:30:00+03:00")  # Tokeo<Wakati, Neno>; also
                                        # "YYYY-MM-DD", a space for T, Z, ±HH:MM or ±HHMM
weka w4 = jaribu kutoka_tarehe(2024, 2, 29)   # Tokeo<Wakati, Neno>: midnight UTC
weka p = tarehe(wakati_sasa)            # Kamusi<Neno, Namba>: mwaka, mwezi, siku, saa,
                                        # dakika, sekunde, siku_ya_wiki (1 = Jumatatu),
                                        # siku_ya_mwaka
weka kuanza = kipima_muda()             # seconds on a clock that never goes back, for
                                        # measuring durations (sasa() follows the wall clock)
lala(1)                                 # sleep for 1 second (argument is seconds, not ms)

weka sasa_namba = majira()             # Namba: raw seconds since epoch (not Wakati)
```

> `majira()` returns `Namba` (raw float seconds). `sasa()` returns `Wakati`; `w.sekunde()`
> (or `sekunde(w)`) gives its seconds since the epoch. Dates follow the proleptic Gregorian
> calendar (leap years included, times before 1970 too) in UTC; `umbiza_eneo` shows another
> time zone by its offset in minutes — there is no time-zone database.

**Maadili ya majira** — available ambiently:

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

jaribu unda_saraka("kumbukumbu/2026")      # makes missing parents too
weka majina = jaribu orodha_saraka("kumbukumbu")   # Orodha<Neno>, sorted
jaribu nakili("a.txt", "b.txt")             # Tokeo<Namba, Neno>: bytes copied
jaribu badili_jina("b.txt", "c.txt")
ni_saraka("kumbukumbu")                     # Ukweli (ni_faili for files)
jaribu futa_saraka("kumbukumbu")            # and everything inside it

njia_unganisha("data", "a.txt")             # "data/a.txt" (the platform's separator)
njia_mzazi("/data/a.txt")                   # Chaguo: "/data"
njia_jina("/data/a.txt")                    # Chaguo: "a.txt"
njia_kiendelezi("/data/a.txt")              # Chaguo: "txt"
jaribu njia_kamili("a.txt")                 # Tokeo: the absolute path, links resolved
```

**Maadili ya faili** — available ambiently:

| Jina             | Maelezo                                     |
|------------------|---------------------------------------------|
| `NJIA_SEPARATOR` | Kitenganishi cha njia (`/` au `\` kwenye OS) |

**Kishikizo cha Faili** (handle-based, available ambiently) — kwa matumizi ya
mara kwa mara badala ya kufungua/kufunga faili kila wakati:

```asili
leta faili

weka w = jaribu (faili_fungua("data.txt", "andika"))
jaribu (w.andika("mstari mmoja\n"))
w.funga()

weka r = jaribu (faili_fungua("data.txt", "soma"))
weka maudhui = jaribu (r.soma())
```

`faili_fungua(njia, hali)` inarejesha `Tokeo<Faili, Neno>`; `hali` ni `"soma"`, `"andika"`, au
`"ongeza"`. Kishikizo hufungwa kiotomatiki `w`/`r` yanapotoka nje ya wigo (hata bila `.funga()`
au `tupa` wazi), si tu wakati zinapofungwa kwa mkono — tazama
[faili-mkondo-design.md](../design/faili-mkondo-design.md).

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

1:1 mfumo wa nyuzi za OS halisi (si nyuzi za kijani/mratibu) — `tenda` moja huanzisha uzi mmoja
wa OS. Tazama [concurrency-design.md](../design/concurrency-design.md) kwa uamuzi kamili.

```asili
leta sambamba
leta matumizi

kazi mfanyakazi(tx: NjiaTx<Namba>, n: Namba) -> Tupu {
    weka jumla = n * n
    jaribu (tx.tuma(jumla))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka p = njia()
    weka tx = p.kwanza()
    weka rx = p.pili()

    weka id = jaribu (tenda("mfanyakazi", tx, 5))
    weka jibu = jaribu (rx.pokea())
    chapisha(jibu kama Neno)          # "25"
    jaribu (subiri_tenda(id))
}
```

`tenda(kazi_jina, hoja...)` inatafuta `kazi` ya kiwango cha moduli kwa jina lake (`Neno`, si
rejeleo la moja kwa moja la kazi) na kuianzisha kwenye uzi mpya, ikirejesha `Tokeo<Namba, Neno>`
(kitambulisho cha uzi). **Thamani inayorejeshwa na kazi iliyoanzishwa haiwezi kupitia
`subiri_tenda` moja kwa moja** — `subiri_tenda(id)` inarejesha tu `Tokeo<Tupu, Neno>` (uzi
ulikamilika kwa usalama au ulianguka), si matokeo halisi. Tumia `njia` kutuma matokeo kurudi.

**Hoja za `tenda` lazima ziweze kuvuka nyuzi salama** — `Kasha_GC<T>`, `Faili`, `Mkondo` (au
chochote kinachozibeba) hukataliwa na `Kosa` badala ya kuruhusiwa kimya kimya:

```asili
leta kasha_gc   # Kasha_GC ni ya hiari: haipatikani bila leta

weka g = kasha_gc_unda(1.0)
linganisha tenda("kazi_yoyote", g) {
    Tokeo::Sawa(_) => { }
    Tokeo::Kosa(ujumbe) => { chapisha(ujumbe) }   # "tenda: hoja ina thamani isiyoweza kuvuka nyuzi..."
}
```

**njia (Channel):**

| Njia          | Maelezo                                          |
|---------------|-----------------------------------------------------|
| `njia()`      | Unda jozi mpya ya `NjiaTx<T>`/`NjiaRx<T>` — haiwezi kushindwa |
| `tx.tuma(v)`  | Tuma `v`; `Tokeo<Tupu, Neno>` — `Kosa` ikiwa upande wa kupokea umefungwa |
| `rx.pokea()`  | Pokea (inasubiri); `Tokeo<T, Neno>` — `Kosa` mara zote za kutuma zinapokuwa zimefungwa |

**fungo (Mutex):**

```asili
weka f = jaribu (fungo(0.0))
f.weka(42.0)
chapisha(f.pata() kama Neno)   # "42"
```

| Njia          | Maelezo                                          |
|---------------|-----------------------------------------------------|
| `fungo(v)`    | Funga `v` kwenye `Fungo<T>`; `Tokeo<Fungo<T>, Neno>` |
| `f.pata()`    | Funga, soma, fungua — hatua moja salama         |
| `f.weka(v)`   | Funga, andika, fungua — hatua moja salama       |
| `f.funga()`   | Funga wazi, bila kufungua — kwa shughuli kadhaa zinazohitaji kufungwa pamoja |
| `f.fungua()`  | Fungua; kuita bila `.funga()` iliyotangulia ni kosa la programu (huripotiwa kama paparika) |

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

### Kumbukumbu\<T\> — Heap Box

Always in scope (via `msingi`, no `leta` inahitajika) — jozi na `Kasha_GC<T>`, lakini **hakuna**
sifa ya kushirikiana wala kubadilisha mahali pale:

```asili
weka k = kumbukumbu_unda(42.0)  # box a value: Kumbukumbu<Namba>
chapisha(k.pata() kama Neno)     # "42" — a clone of the boxed value
```

| Method | Effect |
|---|---|
| `kumbukumbu_unda(v)` | Box `v`, returning a new `Kumbukumbu<T>`. |
| `.pata()` | Read a clone of the boxed value. |

**Hakuna `.weka()`** — `Kumbukumbu<T>` haishirikiani kama `Kasha_GC<T>`; kuita njia yoyote
hupokea nakala ya kishikizo, hivyo kuibadilisha ndani ya njia hiyo hakuonekani kwenye jina la
asili. Ili kubadilisha, kabidhi upya jina zima: `weka k = kumbukumbu_unda(thamani_mpya)`. Tumia
`Kasha_GC<T>` badala yake ikiwa unahitaji kubadilisha mahali pale kupitia vishikizo vingi.

---

## Kuunda Moduli za Mradi Wako

Asili projects are single-file at the entry point (`src/kuu.as`). Cross-file modules are currently imported via `leta` only for the standard library. For multi-file projects, use separate project directories and the `ongeza` dependency system.
