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
| `s.herufi_kwa(i)`       | `Chaguo<Herufi>` | Character at grapheme position `i` (like `urefu()` counts), or `Hamna` if out of range |

### Mifano

```asili
weka s = "Habari Dunia"

s.urefu()                        # 12
s.biti_ngapi()                   # 12 (all ASCII here)
s.kata(0, 6)                     # "Habari"
s.tafuta("Dunia")                # Chaguo(Kuna(Namba(7.0))) — debug-format includes type wrappers
s.tafuta("xyz")                  # Chaguo(Hamna)
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
s.urefu()          # 4  (c, a, f, é — four graphemes)
s.biti_ngapi()     # 5  (é is two bytes in UTF-8)
s.kata(0, 3)       # "caf" — byte slice, not grapheme slice; cutting at byte 4 lands
                   #        mid-character (é starts at byte 3) and produces invalid UTF-8
s.herufi_kwa(3)    # Chaguo(Kuna(Herufi('é'))) — grapheme-indexed, so this is always
                   #        the right position regardless of byte width; use this
                   #        instead of `.kata()` when inspecting one character at a time
                   #        (e.g. writing a tokenizer)
s.herufi_kwa(99)   # Chaguo(Hamna) — out of range, not a panic
```

---

## Orodha — List Methods

| Njia              | Matokeo        | Maelezo                                         |
|-------------------|----------------|-------------------------------------------------|
| `a.urefu()`       | `Namba`        | Number of elements                              |
| `a.clona()`       | `Orodha<T>`    | Deep copy of the list                           |
| `a.ongeza(x)`     | `Tupu`         | Append `x` to end (mutates in place)            |
| `a.ondoa(i)`      | `Chaguo<T>`    | Remove and return element at index `i`; `Hamna` if out of bounds |
| `a.kila_mmoja(f)` | `Tupu`         | Call function `f` (passed as a **string literal** naming it) for each element; no-op if no argument |
| `a.ingiza(i, v)`  | `Tupu`         | Replace the element at index `i` with `v` (mutates in place); panics if `i` is out of bounds — does **not** grow the list. `a[i] = v` desugars to this call. |
| `a[i]?`           | `T`            | Index with error propagation (not a method — see below) |

### Mifano

```asili
weka a = orodha(10, 20, 30, 40)

a.urefu()         # 4
a.ongeza(50)
a.urefu()         # 5

weka kipengee = a.ondoa(1)   # Chaguo(Kuna(20)); a is now [10, 30, 40, 50]
weka nje = a.ondoa(99)       # Chaguo(Hamna) — out of bounds

# kila_mmoja with a named function — pass the name as a string, not a bare identifier
kazi chapisha_kitu(x: Namba) -> Tupu {
  chapisha(x kama Neno)
}
a.kila_mmoja("chapisha_kitu")
```

### Kufikia Kipengele (Indexing)

```asili
weka a = orodha(5, 10, 15)
weka x = a[0]?    # 5   — propagates if out of bounds
weka y = a[2]?    # 15

# Index assign
a[1] = 99         # a is now [5, 99, 15]
a[10] = 1         # panics: "ingiza: index nje ya mipaka" — out-of-bounds assign does not grow
                  # the list; use .ongeza(x) to append instead
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
| `m.idadi()`        | `Namba`        | Number of entries                                    |
| `m.clona()`        | `Kamusi<K,V>`  | Deep copy                                            |

### Mifano

```asili
weka m = kamusi_tupu()
m.ingiza("jina", "Amara")
m.ingiza("umri", 30)

m.pata("jina").angu("")        # "Amara" — unwrap with .angu() first;
                                # `kama Neno` on a Chaguo does NOT unwrap it
                                # (produces a debug-format string instead)
m.pata("nchi").angu("Kenya")   # "Kenya" (default)
m.vipo("jina")                 # kweli
m.vipo("xyz")                  # si_kweli

weka funguo = m.funguo()       # ["jina", "umri"] (order not guaranteed)

# Key-based assignment
m["toleo"] = 2                 # insert/update via index syntax

# Key-based read: NOT the same wrapping as .pata()
m["jina"]                      # "Amara" — plain value (or bare Hamna if missing)
m.pata("jina")                 # Chaguo(Kuna(Neno("Amara"))) — wrapped
```

**Note:** `m[k]` (direct index-read) returns the plain value (or `Hamna` if the key is
missing) — it does **not** wrap the result in `Chaguo` the way `m.pata(k)` does. The two are
not interchangeable despite reading the same key.

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

`Tokeo<T,E>` is either `Ok(value)` or `Err(error)`. Match it directly with `Tokeo::Sawa(v)`/
`Tokeo::Kosa(e) =>` arms in `linganisha`, or use the methods below.

| Njia            | Matokeo  | Maelezo                                            |
|-----------------|----------|----------------------------------------------------|
| `t.angu(mbadala)` | `T`    | Unwrap the Ok value or return `mbadala` on error   |
| `t.ni_sawa()`   | `Ukweli` | `kweli` if Ok                                     |
| `t.ni_kosa()`   | `Ukweli` | `kweli` if Err                                    |
| `t.kosa()`      | `E`      | Get the error value (panics if called on Ok)       |

```asili
leta hisabati

weka j = gawio(10, 2)   # Tokeo::Sawa(5)
j.ni_sawa()             # kweli
j.angu(0)               # 5

weka k = gawio(10, 0)   # Tokeo::Kosa("gawio kwa sifuri")
k.ni_kosa()             # kweli
k.kosa()                # "gawio kwa sifuri"
k.angu(0)               # 0 (default)
```

Note: `weka k = gawio(10, 0)` alone doesn't compile — a `Tokeo` binding must be consumed via
`linganisha`/`?`/`jaribu` in the scope it's created (`SEM048`); the snippet above is
illustrative of the method behavior, not a standalone compiling program. See
[05-makosa.md](05-makosa.md) for a full compiling example.

---

## Wakati — Free Functions

`Wakati` is a plain opaque time value with **no methods of its own** — `w.sekunde()` does not
exist and fails to compile (`SEM039: aina 'Wakati' haina njia`). Use the free functions instead:

| Kazi            | Matokeo  | Maelezo                   |
|-----------------|----------|---------------------------|
| `sekunde(w)`    | `Namba`  | Seconds since Unix epoch  |
| `umbiza(w)`     | `Neno`   | Formatted string (see caveat below) |

```asili
leta majira

weka w = sasa()
sekunde(w)    # e.g. 1718000000.0
```

**Known bug:** `umbiza()`'s calendar-date formatting is inaccurate (fixed 30-day months, no
leap-year handling) — see [implementation-status.md](../design/implementation-status.md).

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
| `kamusi()`        | `Kamusi<K,V>` | Empty dictionary — takes **no** arguments today (`kamusi(k, v, ...)` does not build a populated dict, despite what the name suggests; any argument is a compile error). Identical to `kamusi_tupu()`. |
| `kamusi_tupu()`   | `Kamusi<K,V>` | Empty dictionary                    |
| `jozi(a, b)`      | `Jozi<A,B>`   | Create a pair                       |

```asili
weka ok_val = tokeo(42)        # Tokeo::Sawa(42)
weka err_val = kosa("oops")   # Tokeo::Kosa("oops")
weka some_val = chaguo(5)     # Chaguo::Kuna(5)

weka m = kamusi_tupu()
m.ingiza("a", 1)
m.ingiza("b", 2)               # {"a": 1, "b": 2} — build via .ingiza(), not a variadic constructor
```
