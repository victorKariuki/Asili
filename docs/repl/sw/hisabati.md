# Hisabati — Msaada wa REPL

**Kumbuka:** Kazi za hisabati zinapatikana moja kwa moja kwenye REPL (hauitaji `leta`).

## Hesabu za Msingi

| Kazi                | Maelezo                                | Mfano                      |
|---------------------|-----------------------------------------|----------------------------|
| `jumla(a, b)`       | Jumlisho                               | `jumla(2, 3)` → `5`       |
| `tofauti(a, b)`     | Toa                                    | `tofauti(10, 4)` → `6`    |
| `zao(a, b)`         | Zidisha                                | `zao(3, 5)` → `15`        |
| `gawio(a, b)`       | Mgawanyo; `Tokeo` (Kosa ikiwa b = 0)   | `jaribu gawio(10, 2)` → `5`|
| `baki(a, b)`        | Baki (`a % b`)                         | `baki(10, 3)` → `1`       |
| `absolute(n)`/`abs(n)` | Thamani kamili                      | `absolute(-7)` → `7`      |
| `ishara(n)`         | Ishara: -1, 0, au 1                    | `ishara(-5)` → `-1`       |
| `kipeo(n, p)`       | Kipeo `n ** p`; `Tokeo`                | `jaribu kipeo(2, 8)` → `256`|
| `kipeuo2(n)`        | `n ** 2`                               |                            |
| `kipeuo3(n)`        | `n ** 3`                               |                            |
| `mizizi(n)`         | Mzizi wa mraba; `Tokeo` (Kosa ikiwa n < 0) | `jaribu mizizi(9)` → `3`  |
| `mzizi(x, n)`       | Mzizi wa n wa `x`; `Tokeo` (Kosa ikiwa `n <= 0`, au `n` ni shufwa na `x < 0`) | `jaribu mzizi(27, 3)` → `3` |
| `faktoriali(n)`     | `n!`; `Tokeo` (Kosa ikiwa `n` ni hasi au si namba kamili) | `jaribu faktoriali(5)` → `120` |

## Upeo na Vikwazo

| Kazi                | Maelezo                                |
|---------------------|-----------------------------------------|
| `kubwa(a, b)`       | Kikubwa kati ya namba mbili            |
| `ndogo(a, b)`       | Kidogo kati ya namba mbili             |
| `upeo(n, p)`        | Kipeo `n ** p`; `Tokeo` (Kosa ikiwa matokeo si namba dhabiti — **si** jina mbadala la `kubwa`, licha ya jina) | `jaribu upeo(3, 7)` → `2187` |
| `kikwazo(n, kima, juu)` | Bana `n` kati ya `kima` na `juu`  |
| `haipot(a, b)`      | Hipotenuse: `√(a² + b²)`               |
| `upeo_wa_e(n)`      | `e^n` (kipeo cha e)                    |
| `expm1(n)`          | `e^n - 1` (sahihi zaidi kwa `n` ndogo) |
| `punguza(n)`        | Kata kuelekea sifuri (hoja moja, si mgawanyo wa sakafu wa namba mbili) | `punguza(3.7)` → `3`, `punguza(-3.7)` → `-3` |

## Sakafu, Dari, Duara

| Kazi          | Maelezo                          | Mfano                   |
|---------------|------------------------------------|-------------------------|
| `sakafu(n)`   | Sakafu (zungusha chini)          | `sakafu(3.7)` → `3`    |
| `dari(n)`     | Dari (zungusha juu)               | `dari(3.2)` → `4`      |
| `duara(n)`    | Zungusha hadi namba iliyo karibu  | `duara(3.5)` → `4`     |
| `duara_maeneo(x, m)` | Zungusha `x` hadi tarakimu `m` za desimali (si kazi ya eneo la duara, licha ya jina) | `duara_maeneo(3.14159, 2)` → `3.14` |

## Logarithimu

Zote nne zinarejesha `Tokeo` (zimehakikiwa kidomeni — Kosa nje ya wigo halali wa kila kazi).

| Kazi        | Maelezo                      | Domeni (vinginevyo Kosa) |
|-------------|--------------------------------|----------------------------|
| `logi(n)`   | Logarithi ya asili (ln)      | `n > 0`                    |
| `logi2(n)`  | Logarithi msingi 2            | `n > 0`                    |
| `logi10(n)` | Logarithi msingi 10           | `n > 0`                    |
| `logi1p(n)` | `ln(1 + n)` — sahihi karibu na 0 | `n > -1`               |

## Trigonometria (Radiani)

| Kazi              | Maelezo                       | `Tokeo`? |
|-------------------|----------------------------------|----------|
| `sini(x)`         | Sine                            | Hapana — `Namba` tu |
| `kosini(x)`       | Cosine                          | Hapana — `Namba` tu |
| `tanjenti(x)`     | Tangent                         | Hapana — `Namba` tu |
| `asini(x)`        | Arc sine                        | Ndiyo — Kosa ikiwa `x` iko nje ya `[-1, 1]` |
| `akosini(x)`      | Arc cosine                      | Ndiyo — Kosa ikiwa `x` iko nje ya `[-1, 1]` |
| `atanjenti(x)`    | Arc tangent                     | Ndiyo — daima `Ok` (hakuna kikwazo cha domeni), bado imefungwa kwa `Tokeo` |
| `atanjenti2(y, x)`| Arc tangent ya y/x              | Hapana — `Namba` tu |
| `sini_h(x)`       | Sine ya hyperbolic              | Hapana — `Namba` tu |
| `kosini_h(x)`     | Cosine ya hyperbolic            | Hapana — `Namba` tu |
| `tanjenti_h(x)`   | Tangent ya hyperbolic           | Hapana — `Namba` tu |
| `asini_h(x)`      | Inverse ya sine ya hyperbolic   | Ndiyo — daima `Ok`, bado imefungwa kwa `Tokeo` |
| `akosini_h(x)`    | Inverse ya cosine ya hyperbolic | Ndiyo — Kosa ikiwa `x < 1` |
| `atanjenti_h(x)`  | Inverse ya tangent ya hyperbolic | Ndiyo — Kosa ikiwa `x` iko nje ya `(-1, 1)` |

Tumia `jaribu` kufungua kazi yoyote inayorejesha `Tokeo` katika jedwali hili kabla ya kutumia
matokeo kama `Namba` tupu — mf. `jaribu asini(0.5)`, si `asini(0.5)` moja kwa moja.

## Ubadilishaji wa Pembe

| Kazi                | Maelezo                   |
|---------------------|----------------------------|
| `kwenda_radiani(d)` | Nyuzi → radiani            |
| `kwenda_nyuzi(r)`   | Radiani → nyuzi            |

## Nasibu

| Kazi              | Maelezo                                   |
|-------------------|---------------------------------------------|
| `nasibu()`        | Namba nasibu ya desimali katika `[0.0, 1.0)` |
| `nasibu_chini(min, max)` | Namba nasibu ya desimali katika `[min, max)` (hoja 2, si 1) |

## Ukaguzi wa Aina ya Namba

| Kazi            | Maelezo                        |
|-----------------|-----------------------------------|
| `ni_namba(n)`   | `kweli` ikiwa si NaN wala Inf   |
| `si_namba(n)`   | `kweli` ikiwa ni NaN            |
| `ni_ukomo(n)`   | `kweli` ikiwa ni ±Inf           |

## Mfano

```
> jaribu mizizi(16)
Namba(4.0)
> kubwa(3, 7)
Namba(7.0)
> sini(kwenda_radiani(90))
Namba(1.0)
> nasibu()
Namba(0.7342...)
> jaribu faktoriali(6)
Namba(720.0)
```
