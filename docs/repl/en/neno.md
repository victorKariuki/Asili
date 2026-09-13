# Neno (string) — REPL help

`Neno` is a sequence of UTF-8 characters.

## Creating

```
> weka s = "Habari"
> weka tupu = ""
```

## Methods

| Expression          | Description                               | Example                        |
|----------------------|--------------------------------------------|---------------------------------|
| `s.urefu()`         | Number of grapheme clusters               | `"café".urefu()` → `4`        |
| `s.biti_ngapi()`    | Length in bytes (UTF-8)                   | `"é".biti_ngapi()` → `2`      |
| `s.kata(a, b)`      | Byte slice from `a` to `b`                | `"hello".kata(1, 4)` → `"ell"`|
| `s.tafuta(p)`       | Position of `p` within `s` (Chaguo)       | `"hello".tafuta("ll")` → `Chaguo(Kuna(Namba(2.0)))` |
| `s.unganisha(kip)`  | Append `kip`                              | `"a".unganisha("-")` → `"a-"` |
| `s.clona()`         | Copy the string                           | `"a".clona()` → `"a"`         |
| `s.gawanya(sep)`    | Split by separator; `Orodha<Neno>`        | `"a,b,c".gawanya(",")` → `["a","b","c"]` |
| `s.badilisha(kutoka, kwenda)` | Replace all occurrences         | `"hi Dunia".badilisha("Dunia", "Asili")` → `"hi Asili"` |
| `s.kwa_herufi_ndogo()` | Convert to lowercase                   | `"HABARI".kwa_herufi_ndogo()` → `"habari"` |
| `s.kwa_herufi_kubwa()` | Convert to uppercase                   | `"habari".kwa_herufi_kubwa()` → `"HABARI"` |
| `s.anza_na(p)`      | `kweli` if `s` starts with `p`            | `"Habari".anza_na("Hab")` → `kweli` |
| `s.maliza_na(p)`    | `kweli` if `s` ends with `p`              | `"Habari".maliza_na("ari")` → `kweli` |
| `s.herufi_kwa(i)`   | Character at grapheme position `i` (same counting as `urefu()`); `Chaguo<Herufi>` | `"café".herufi_kwa(3)` → `Chaguo(Kuna(Herufi('é')))` |

## Concatenation

```
> weka a = "Habari"
> weka b = ", Dunia!"
> a + b
Neno("Habari, Dunia!")

> weka n = 42
> "Namba ni: " + (n kama Neno)
Neno("Namba ni: 42")
```

## Comparison

```
> "sawa" == "sawa"
Ukweli(true)
> "tofauti" != "sawa"
Ukweli(true)
```

## Casting

```
> 65 kama Herufi kama Neno
Neno("A")
> kweli kama Neno
Neno("kweli")
```
