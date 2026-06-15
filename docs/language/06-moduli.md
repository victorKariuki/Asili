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
weka d = duara(7)             # 49
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
weka umbizwa = umbiza(wakati_sasa, "%Y-%m-%d")  # formatted string
lala(1000)                              # sleep for 1000 milliseconds

weka sasa_namba = majira()             # Namba: raw seconds since epoch (not Wakati)
```

> `majira()` returns `Namba` (raw float seconds). `sasa()` returns `Wakati` (a structured value with `.sekunde()` method and `umbiza()` support). Use `sasa()` when you need formatting or method access; use `majira()` for simple elapsed-time arithmetic.

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
weka ms = muda_wa_kuanza() # milliseconds since process start
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

---

## Kuunda Moduli za Mradi Wako

Asili projects are single-file at the entry point (`src/kuu.as`). Cross-file modules are currently imported via `leta` only for the standard library. For multi-file projects, use separate project directories and the `ongeza` dependency system.
