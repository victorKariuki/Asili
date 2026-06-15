# Njia za Aina Zilizojengewa Ndani (Built-in Type Methods)

All built-in types support method call syntax: `thamani.njia(hoja...)`.

---

## Neno — String Methods

| Njia                    | Matokeo          | Maelezo                                                    |
|-------------------------|------------------|------------------------------------------------------------|
| `s.urefu()`             | `Namba`          | Number of Unicode grapheme clusters (visible characters)   |
| `s.biti_ngapi()`        | `Namba`          | Byte length (UTF-8 encoded size)                           |
| `s.clona()`             | `Neno`           | Create a copy (needed before passing to a function)        |
| `s.kata(mwanzo, mwisho)`| `Neno`           | Byte slice from `mwanzo` to `mwisho` (exclusive)          |
| `s.tafuta(sub)`         | `Chaguo<Namba>`  | Byte index of first match, or `Hamna`                     |
| `s.unganisha(sep)`      | `Neno`           | Append `sep` to end of string                              |
| `s.gawanya(sep)`        | `Orodha<Neno>`   | Split by separator; returns list of parts                  |
| `s.badilisha(kwa, na)`  | `Neno`           | Replace all occurrences of `kwa` with `na`                |
| `s.kwa_herufi_ndogo()`  | `Neno`           | Convert to lowercase                                       |
| `s.kwa_herufi_kubwa()`  | `Neno`           | Convert to uppercase                                       |
| `s.anza_na(kiambishi)`  | `Ukweli`         | `kweli` if string starts with `kiambishi`                 |
| `s.maliza_na(kiishio)`  | `Ukweli`         | `kweli` if string ends with `kiishio`                     |

### Mifano

```asili
weka s = "Habari Dunia"

s.urefu()                        # 12
s.biti_ngapi()                   # 12 (all ASCII here)
s.kata(0, 6)                     # "Habari"
s.tafuta("Dunia")                # Chaguo(Some(7))
s.tafuta("xyz")                  # Chaguo(None)
s.kwa_herufi_ndogo()             # "habari dunia"
s.kwa_herufi_kubwa()             # "HABARI DUNIA"
s.anza_na("Hab")                 # kweli
s.maliza_na("nia")               # kweli
s.gawanya(" ")                   # ["Habari", "Dunia"]
s.badilisha("Dunia", "Asili")    # "Habari Asili"
```

### Neno na UTF-8

`urefu()` counts visible grapheme clusters; `biti_ngapi()` counts raw bytes. They differ for multibyte characters:

```asili
weka s = "café"
s.urefu()       # 4  (c, a, f, é — four graphemes)
s.biti_ngapi()  # 5  (é is two bytes in UTF-8)
s.kata(0, 4)    # "caf" — byte slice, not grapheme slice!
```

---

## Orodha — List Methods

| Njia              | Matokeo        | Maelezo                                         |
|-------------------|----------------|-------------------------------------------------|
| `a.urefu()`       | `Namba`        | Number of elements                              |
| `a.clona()`       | `Orodha<T>`    | Deep copy of the list                           |
| `a.ongeza(x)`     | `Tupu`         | Append `x` to end (mutates in place)            |
| `a.ondoa(i)`      | `Chaguo<T>`    | Remove and return element at index `i`; `Hamna` if out of bounds |
| `a.kila_mmoja(f)` | `Tupu`         | Call function `f` for each element; no-op if no argument |
| `a[i]?`           | `T`            | Index with error propagation (not a method — see below) |

### Mifano

```asili
weka a = orodha(10, 20, 30, 40)

a.urefu()         # 4
a.ongeza(50)
a.urefu()         # 5

weka kipengee = a.ondoa(1)   # Chaguo(Some(20)); a is now [10, 30, 40, 50]
weka nje = a.ondoa(99)       # Chaguo(None) — out of bounds

# kila_mmoja with a named function
kazi chapisha_kitu(x: Namba) -> Tupu {
  chapisha(x kama Neno)
}
a.kila_mmoja(chapisha_kitu)
```

### Kufikia Kipengele (Indexing)

```asili
weka a = orodha(5, 10, 15)
weka x = a[0]?    # 5   — propagates if out of bounds
weka y = a[2]?    # 15

# Index assign
a[1] = 99         # a is now [5, 99, 15]
```

---

## Kamusi — Dictionary Methods

| Njia               | Matokeo        | Maelezo                                              |
|--------------------|----------------|------------------------------------------------------|
| `m.ingiza(k, v)`   | `Tupu`         | Insert or update key `k` with value `v`              |
| `m.weka_key(k, v)` | `Tupu`         | Alias for `ingiza`                                   |
| `m.pata(k)`        | `Chaguo<V>`    | Get value for key; `Hamna` if missing                |
| `m.vipo(k)`        | `Ukweli`       | `kweli` if key exists                                |
| `m.funguo()`       | `Orodha`       | All keys as a list                                   |
| `m.clona()`        | `Kamusi<K,V>`  | Deep copy                                            |

### Mifano

```asili
weka m = kamusi_tupu()
m.ingiza("jina", "Amara")
m.ingiza("umri", 30)

m.pata("jina") kama Neno       # "Amara"
m.pata("nchi").angu("Kenya")   # "Kenya" (default)
m.vipo("jina")                 # kweli
m.vipo("xyz")                  # si_kweli

weka funguo = m.funguo()       # ["jina", "umri"] (order not guaranteed)

# Key-based assignment
m["toleo"] = 2                 # insert/update via index syntax
```

---

## Jozi — Pair Methods

| Njia         | Matokeo  | Maelezo                    |
|--------------|----------|----------------------------|
| `p.kwanza()` | `A`      | First element of the pair  |
| `p.pili()`   | `B`      | Second element of the pair |
| `p.clona()`  | `Jozi<A,B>` | Deep copy               |

```asili
weka p = jozi("Nairobi", 4000000)
p.kwanza()    # "Nairobi"
p.pili()      # 4000000

weka q = jozi(jozi(1, 2), "ndani")
q.kwanza().kwanza()   # 1
q.kwanza().pili()     # 2
```

---

## Chaguo — Option Methods

`Chaguo<T>` is either a value or `Hamna`.

| Njia                   | Matokeo  | Maelezo                                                    |
|------------------------|----------|------------------------------------------------------------|
| `c.angu(mbadala)`      | `T`      | Unwrap or return `mbadala` if `Hamna`                     |
| `c.ni_po()`            | `Ukweli` | `kweli` if has a value                                    |
| `c.ni_tupu()`          | `Ukweli` | `kweli` if `Hamna`                                        |
| `c.hakikisha(ujumbe)`  | `T`      | Unwrap or panic with `ujumbe` if `Hamna`                  |

```asili
weka m = kamusi_tupu()
m.ingiza("x", 42)

weka c = m.pata("x")
c.ni_po()             # kweli
c.angu(0)             # 42

weka d = m.pata("z")
d.ni_tupu()           # kweli
d.angu(99)            # 99
d.hakikisha("Ufunguo z hauko!")   # panics
```

---

## Tokeo — Result Methods

`Tokeo<T,E>` is either `Ok(value)` or `Kosa(error)`.

| Njia            | Matokeo  | Maelezo                                            |
|-----------------|----------|----------------------------------------------------|
| `t.angu(mbadala)` | `T`    | Unwrap the Ok value or return `mbadala` on error   |
| `t.ni_sawa()`   | `Ukweli` | `kweli` if Ok                                     |
| `t.ni_kosa()`   | `Ukweli` | `kweli` if Kosa                                   |
| `t.kosa()`      | `E`      | Get the error value (panics if called on Ok)       |

```asili
leta hisabati

weka j = gawio(10, 2)   # Tokeo::Ok(5)
j.ni_sawa()             # kweli
j.angu(0)               # 5

weka k = gawio(10, 0)   # Tokeo::Kosa("...")
k.ni_kosa()             # kweli
k.kosa()                # error message string
k.angu(0)               # 0 (default)
```

---

## Wakati — Time Methods

| Njia          | Matokeo  | Maelezo                   |
|---------------|----------|---------------------------|
| `w.sekunde()` | `Namba`  | Seconds since Unix epoch  |

```asili
leta majira

weka w = sasa()
w.sekunde()    # e.g. 1718000000.0
```

---

## Kuunda Tokeo na Chaguo (Constructors via msingi)

These are available without any `leta` (they are prelude builtins):

| Kazi              | Matokeo       | Maelezo                             |
|-------------------|---------------|-------------------------------------|
| `tokeo(v)`        | `Tokeo<T,E>`  | Wrap value as Ok                    |
| `ok(v)`           | `Tokeo<T,E>`  | Alias for `tokeo`                   |
| `kosa(e)`         | `Tokeo<T,E>`  | Wrap error value as Kosa            |
| `chaguo(v)`       | `Chaguo<T>`   | Wrap value as Some                  |
| `orodha(...)`     | `Orodha<T>`   | Build list from arguments           |
| `kamusi(k,v,...)`  | `Kamusi<K,V>` | Build dict from alternating k,v pairs |
| `kamusi_tupu()`   | `Kamusi<K,V>` | Empty dictionary                    |
| `jozi(a, b)`      | `Jozi<A,B>`   | Create a pair                       |

```asili
weka ok_val = tokeo(42)        # Tokeo::Ok(42)
weka err_val = kosa("oops")   # Tokeo::Kosa("oops")
weka some_val = chaguo(5)     # Chaguo::Some(5)

weka m = kamusi("a", 1, "b", 2)   # {"a": 1, "b": 2}
```
