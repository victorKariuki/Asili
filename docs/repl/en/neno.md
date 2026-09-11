# Neno (string) — REPL help

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
| `s.kata(a, b)`      | Kata sehemu ya baiti kutoka `a` hadi `b`  | `"hello".kata(1, 4)` → `"ell"`|
| `s.tafuta(p)`       | Nafasi ya `p` ndani ya `s` (Chaguo)       | `"hello".tafuta("ll")` → `2`  |
| `s.unganisha(kip)`  | Unganisha na kiungo                        | `"a".unganisha("-")` → `"a-"` |
| `s.clona()`         | Nakala ya neno                            | `"a".clona()` → `"a"`         |

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
