# Neno — Msaada wa REPL

Neno ni mfululizo wa herufi za UTF-8.

## Kuunda

```
> weka s = "Habari"
> weka tupu = ""
```

## Njia

| Usemi               | Maelezo                                   | Mfano                          |
|---------------------|-------------------------------------------|--------------------------------|
| `s.urefu()`         | Idadi ya grapheme clusters                | `"café".urefu()` → `4`        |
| `s.biti_ngapi()`    | Urefu kwa baiti (UTF-8)                   | `"é".biti_ngapi()` → `2`      |
| `s.kata(a, b)`      | Herufi kutoka nafasi `a` hadi `b` (kama `urefu()` inavyohesabu) | `"café".kata(1, 3)` → `"af"`|
| `s.tafuta(p)`       | Nafasi ya herufi ya `p` ndani ya `s` (Chaguo) | `"hello".tafuta("ll")` → `Chaguo(Kuna(Namba(2.0)))` |
| `s.unganisha(kip)`  | Unganisha na kiungo                        | `"a".unganisha("-")` → `"a-"` |
| `s.clona()`         | Nakala ya neno                            | `"a".clona()` → `"a"`         |
| `s.gawanya(sep)`    | Gawanya kwa kitenganishi; `Orodha<Neno>`  | `"a,b,c".gawanya(",")` → `["a","b","c"]` |
| `s.badilisha(kutoka, kwenda)` | Badilisha matukio yote               | `"hi Dunia".badilisha("Dunia", "Asili")` → `"hi Asili"` |
| `s.kwa_herufi_ndogo()` | Badilisha kuwa herufi ndogo             | `"HABARI".kwa_herufi_ndogo()` → `"habari"` |
| `s.kwa_herufi_kubwa()` | Badilisha kuwa herufi kubwa             | `"habari".kwa_herufi_kubwa()` → `"HABARI"` |
| `s.anza_na(p)`      | `kweli` ikiwa `s` inaanza na `p`          | `"Habari".anza_na("Hab")` → `kweli` |
| `s.maliza_na(p)`    | `kweli` ikiwa `s` inamalizika na `p`      | `"Habari".maliza_na("ari")` → `kweli` |
| `s.herufi_kwa(i)`   | Herufi katika nafasi ya `i` (kama `urefu()` inavyohesabu); `Chaguo<Herufi>` | `"café".herufi_kwa(3)` → `Chaguo(Kuna(Herufi('é')))` |
| `s.safisha()`       | Ondoa nafasi tupu mwanzoni na mwishoni (`safisha_mwanzo`, `safisha_mwisho`: upande mmoja) | `"  sawa ".safisha()` → `"sawa"` |
| `s.jaza_kushoto(n, h)` | Jaza kwa herufi `h` upande wa kushoto hadi herufi `n` (`jaza_kulia`: kulia) | `"7".jaza_kushoto(3, "0")` → `"007"` |
| `s.jaza(orodha)`    | Weka thamani za orodha mahali pa kila `{}` | `"{} ana {}".jaza(["Amara", 30])` → `"Amara ana 30"` |
| `s.herufi()`        | Herufi zote kama `Orodha<Neno>`           | `"abc".herufi()` → `["a","b","c"]` |
| `s.mistari()`       | Mistari yote kama `Orodha<Neno>`          | `"a\nb".mistari()` → `["a","b"]` |
| `s.geuza()`         | Herufi kwa mpangilio wa nyuma             | `"abc".geuza()` → `"cba"` |
| `s.misimbo()`       | Namba za Unicode za herufi zote           | `"A".misimbo()` → `[65]` |
| `s.kwa_namba()`     | Namba iliyoandikwa katika neno; `Tokeo` (kosa ikiwa si namba) | `"2.5".kwa_namba()` → `Tokeo(Sawa(Namba(2.5)))` |

## Kushirikiana

```
> weka a = "Habari"
> weka b = ", Dunia!"
> a + b
Neno("Habari, Dunia!")

> weka n = 42
> "Namba ni: " + (n kama Neno)
Neno("Namba ni: 42")
```

## Ulinganisho

```
> "sawa" == "sawa"
Ukweli(true)
> "tofauti" != "sawa"
Ukweli(true)
```

## Kubadilisha Aina

```
> 65 kama Herufi kama Neno
Neno("A")
> kweli kama Neno
Neno("kweli")
```
