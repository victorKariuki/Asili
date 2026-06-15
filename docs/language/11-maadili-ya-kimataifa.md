# Maadili ya Kimataifa (Global Constants)

These constants are always in scope — no `leta` required. They are seeded by the runtime before any user code runs.

## Ukweli (Boolean)

| Jina          | Aina    | Thamani |
|---------------|---------|---------|
| `KWELI`       | Ukweli  | kweli   |
| `SIYO_KWELI`  | Ukweli  | si_kweli|

```asili
ikiwa KWELI { chapisha("daima") }
weka x: Ukweli = SIYO_KWELI
```

## Tupu

| Jina   | Aina  | Thamani |
|--------|-------|---------|
| `TUPU` | Tupu  | tupu    |

```asili
weka hakuna: Tupu = TUPU
```

## Namba Maalum (Special Numeric Values)

| Jina        | Aina  | Thamani | Maelezo                        |
|-------------|-------|---------|--------------------------------|
| `Ukomo`     | Namba | ∞       | Infinity chanya                |
| `Siyo_Namba`| Namba | NaN     | Not a Number                   |
| `INF`       | Namba | ∞       | Alias ya `Ukomo`               |
| `NAN`       | Namba | NaN     | Alias ya `Siyo_Namba`          |

```asili
chapisha(Ukomo)               # inf
chapisha(-Ukomo)              # -inf
chapisha(Siyo_Namba)          # NaN
chapisha(Siyo_Namba == Siyo_Namba kama Neno)  # si_kweli — NaN ≠ NaN
chapisha(1.0 / 0.0 == Ukomo kama Neno)        # kweli
```

## Maadili ya Hisabati (Math Constants)

| Jina        | Thamani               | Maelezo                      |
|-------------|-----------------------|------------------------------|
| `PI`        | 3.14159265358979…     | π                            |
| `E`         | 2.71828182845904…     | Namba ya Euler               |
| `TAU`       | 6.28318530717958…     | 2π (mzunguko kamili)         |
| `PHI`       | 1.61803398874989…     | Uwiano wa Dhahabu (φ)        |
| `LN2`       | 0.6931471805599453    | ln(2)                        |
| `LN10`      | 2.302585092994046     | ln(10)                       |
| `LOG2E`     | 1.4426950408889634    | log₂(e)                      |
| `LOG10E`    | 0.4342944819032518    | log₁₀(e)                     |
| `KIPEUO1_2` | 0.7071067811865476    | 1/√2                         |
| `KIPEUO2`   | 1.4142135623730951    | √2                           |
| `KIPEUO3`   | 1.7320508075688772    | √3                           |
| `KIPEUO5`   | 2.23606797749979      | √5                           |
| `EPSILON`   | 2.220446049250313e-16 | Tofauti ndogo zaidi ya float |

```asili
weka eneo = PI * duara(5)        # eneo la duara, r=5
weka g = PHI * PHI - PHI         # ≈ 1.0 (golden ratio identity)
```

## Maadili ya Mfumo (System Constants)

| Jina                | Aina  | Maelezo                                    |
|---------------------|-------|--------------------------------------------|
| `TOLEO`             | Neno  | Toleo la sasa la Asili (e.g. "0.1.0")     |
| `JINA_OS`           | Neno  | Jina la OS (e.g. "linux", "macos")        |
| `SEKUNDE_KWA_SIKU`  | Namba | Sekunde kwa siku (86400)                  |
| `MWANZO_WA_ZAMANI`  | Namba | Unix epoch kama Namba (0)                 |
| `NJIA_SEPARATOR`    | Neno  | Kitenganishi cha njia ("/" au "\\")       |

```asili
chapisha("Toleo: " + TOLEO)
chapisha("Mfumo: " + JINA_OS)

weka siku_kwa_wiki = SEKUNDE_KWA_SIKU * 7
```

> These are truly global — they are the same constants exported by the stdlib modules (`hisabati`, `mfumo`, `majira`, `faili`), but seeded directly into the global environment so they work even without any `leta` statement.
