# Hisabati (math) — REPL help

**Note:** Math functions are auto-available in the REPL (no `leta` needed).

## Hesabu za Msingi

| Kazi                | Maelezo                                | Mfano                      |
|---------------------|----------------------------------------|----------------------------|
| `jumla(a, b)`       | Addition                               | `jumla(2, 3)` → `5`       |
| `tofauti(a, b)`     | Subtraction                            | `tofauti(10, 4)` → `6`    |
| `zao(a, b)`         | Multiplication                         | `zao(3, 5)` → `15`        |
| `gawio(a, b)`       | Division; `Tokeo` (Kosa if b = 0)      | `jaribu gawio(10, 2)` → `5`|
| `baki(a, b)`        | Remainder (`a % b`)                    | `baki(10, 3)` → `1`       |
| `absolute(n)`/`abs(n)` | Absolute value                      | `absolute(-7)` → `7`      |
| `ishara(n)`         | Sign: -1, 0, or 1                      | `ishara(-5)` → `-1`       |
| `kipeo(n, p)`       | Power `n ** p`; `Tokeo`               | `jaribu kipeo(2, 8)` → `256`|
| `kipeuo2(n)`        | `n ** 2`                               |                            |
| `kipeuo3(n)`        | `n ** 3`                               |                            |
| `mizizi(n)`         | Square root; `Tokeo` (Kosa if n < 0)  | `jaribu mizizi(9)` → `3`  |
| `duara(r)`          | `r * r` (area helper)                  |                            |
| `duara_maeneo(r)`   | Circle area: `π * r²`                  |                            |
| `faktoriali(n)`     | `n!`                                   | `faktoriali(5)` → `120`   |

## Upeo na Vikwazo

| Kazi                | Maelezo                                |
|---------------------|----------------------------------------|
| `kubwa(a, b)`       | Maximum of two numbers                 |
| `ndogo(a, b)`       | Minimum of two numbers                 |
| `upeo(a, b)`        | Alias for `kubwa`                      |
| `kikwazo(n, kima, juu)` | Clamp `n` between `kima` and `juu` |
| `haipot(a, b)`      | Hypotenuse: `√(a² + b²)`              |
| `upeo_wa_e(n)`      | `e^n` (exp)                            |
| `expm1(n)`          | `e^n - 1` (accurate for small `n`)     |
| `punguza(a, b)`     | Floor division                         |

## Sakafu, Dari, Duara

| Kazi          | Maelezo                       | Mfano                   |
|---------------|-------------------------------|-------------------------|
| `sakafu(n)`   | Floor (round down)            | `sakafu(3.7)` → `3`    |
| `dari(n)`     | Ceiling (round up)            | `dari(3.2)` → `4`      |
| `duara(n)`    | Round to nearest              | `duara(3.5)` → `4`     |

## Logarithimu

| Kazi        | Maelezo             |
|-------------|---------------------|
| `logi(n)`   | Natural log (ln)    |
| `logi2(n)`  | Log base 2          |
| `logi10(n)` | Log base 10         |
| `logi1p(n)` | `ln(1 + n)` — accurate near 0 |

## Trigonometria (Radiani)

| Kazi              | Maelezo               |
|-------------------|-----------------------|
| `sini(x)`         | Sine                  |
| `kosini(x)`       | Cosine                |
| `tanjenti(x)`     | Tangent               |
| `asini(x)`        | Arc sine              |
| `akosini(x)`      | Arc cosine            |
| `atanjenti(x)`    | Arc tangent           |
| `atanjenti2(y, x)`| Arc tangent of y/x    |
| `sini_h(x)`       | Hyperbolic sine       |
| `kosini_h(x)`     | Hyperbolic cosine     |
| `tanjenti_h(x)`   | Hyperbolic tangent    |
| `asini_h(x)`      | Inverse hyperbolic sine |
| `akosini_h(x)`    | Inverse hyperbolic cosine |
| `atanjenti_h(x)`  | Inverse hyperbolic tangent |

## Ubadilishaji wa Pembe

| Kazi                | Maelezo                   |
|---------------------|---------------------------|
| `kwenda_radiani(d)` | Degrees → radians         |
| `kwenda_nyuzi(r)`   | Radians → degrees         |

## Nasibu

| Kazi              | Maelezo                         |
|-------------------|---------------------------------|
| `nasibu()`        | Random float in `[0.0, 1.0)`   |
| `nasibu_chini(n)` | Random float in `[0.0, n)`     |

## Ukaguzi wa Aina ya Namba

| Kazi            | Maelezo                    |
|-----------------|----------------------------|
| `ni_namba(n)`   | `kweli` if not NaN or Inf  |
| `si_namba(n)`   | `kweli` if NaN             |
| `ni_ukomo(n)`   | `kweli` if ±Inf            |

## Mfano

```
> leta hisabati
> jaribu mizizi(16)
Namba(4.0)
> kubwa(3, 7)
Namba(7.0)
> sini(kwenda_radiani(90))
Namba(1.0)
> nasibu()
Namba(0.7342...)
> faktoriali(6)
Namba(720.0)
```
