# Njia za Aina Zilizojengewa Ndani (Built-in Type Methods)

All built-in types support method call syntax: `thamani.njia(hoja...)`.

---

## Neno — String Methods

| Njia                    | Matokeo          | Maelezo                                                    |
|-------------------------|------------------|------------------------------------------------------------|
| `s.urefu()`             | `Namba`          | Number of Unicode grapheme clusters (visible characters)   |
| `s.biti_ngapi()`        | `Namba`          | Byte length (UTF-8 encoded size)                           |
| `s.clona()`             | `Neno`           | Create a copy (needed before passing to a function)        |
| `s.kata(mwanzo, mwisho)`| `Neno`           | Characters from `mwanzo` to `mwisho` (exclusive; `mwisho` optional, both clamped to the text) |
| `s.tafuta(sub)`         | `Chaguo<Namba>`  | Character position of the first match, or `Hamna`         |
| `s.unganisha(sep)`      | `Neno`           | Append `sep` to end of string                              |
| `s.gawanya(sep)`        | `Orodha<Neno>`   | Split by separator; returns list of parts                  |
| `s.badilisha(kwa, na)`  | `Neno`           | Replace all occurrences of `kwa` with `na`                |
| `s.kwa_herufi_ndogo()`  | `Neno`           | Convert to lowercase                                       |
| `s.kwa_herufi_kubwa()`  | `Neno`           | Convert to uppercase                                       |
| `s.tupu()`              | `Ukweli`         | Whether the string has no characters                       |
| `s.ina(sub)`            | `Ukweli`         | Whether the string contains `sub`                          |
| `s.hesabu(sub)`         | `Namba`          | Count non-overlapping occurrences of `sub`                 |
| `s.rudia(n)`            | `Neno`           | Repeat the string `n` times                                |
| `s.anza_na(kiambishi)`  | `Ukweli`         | `kweli` if string starts with `kiambishi`                 |
| `s.maliza_na(kiishio)`  | `Ukweli`         | `kweli` if string ends with `kiishio`                     |
| `s.herufi_kwa(i)`       | `Chaguo<Herufi>` | Character at grapheme position `i` (like `urefu()` counts), or `Hamna` if out of range |
| `s.safisha()`           | `Neno`           | Without whitespace at either end (`safisha_mwanzo` / `safisha_mwisho`: one end only) |
| `s.jaza_kushoto(n, h)`  | `Neno`           | Padded on the left with the one character `h` (default a space) to `n` characters; `jaza_kulia` pads on the right; longer text is unchanged |
| `s.jaza(orodha)`        | `Neno`           | Each `{}` replaced by the next item's text (`{{` and `}}` are literal braces); the counts must match |
| `s.herufi()`            | `Orodha<Neno>`   | The characters, each as a `Neno`                          |
| `s.mistari()`           | `Orodha<Neno>`   | The lines (separated by `\n` or `\r\n`)                   |
| `s.geuza()`             | `Neno`           | The characters in reverse order                           |
| `s.misimbo()`           | `Orodha<Namba>`  | The Unicode code point of every code point in the text    |
| `s.kwa_namba()`         | `Tokeo<Namba, Neno>` | The number the text spells (spaces around it allowed), or an error — unlike `kama Namba`, which gives 0 for text that is not a number |

### Mifano

```asili
weka s = "Habari Dunia"

s.urefu()                        # 12
s.biti_ngapi()                   # 12 (all ASCII here)
s.kata(0, 6)                     # "Habari"
s.kata(7)                        # "Dunia"
s.tafuta("Dunia")                # Chaguo(Kuna(Namba(7.0))) — debug-format includes type wrappers
s.tafuta("xyz")                  # Chaguo(Hamna)
s.kwa_herufi_ndogo()             # "habari dunia"
s.kwa_herufi_kubwa()             # "HABARI DUNIA"
s.anza_na("Hab")                 # kweli
s.maliza_na("nia")               # kweli
s.gawanya(" ")                   # ["Habari", "Dunia"]
s.badilisha("Dunia", "Asili")    # "Habari Asili"
"  sawa  ".safisha()             # "sawa"
"7".jaza_kushoto(3, "0")         # "007"
"{} ana miaka {}".jaza(["Amara", 30])   # "Amara ana miaka 30"
"2.5".kwa_namba()                # Tokeo(Sawa(Namba(2.5)))
```

### Neno na UTF-8

`urefu()` counts visible grapheme clusters; `biti_ngapi()` counts raw bytes. They differ for multibyte characters:

```asili
weka s = "café"
s.urefu()          # 4  (c, a, f, é — four graphemes)
s.biti_ngapi()     # 5  (é is two bytes in UTF-8)
s.kata(0, 3)       # "caf" — positions count characters, like urefu()
s.kata(3)          # "é"
s.tafuta("é")      # Chaguo(Kuna(Namba(3.0))) — the character position, not the byte offset
s.herufi_kwa(3)    # Chaguo(Kuna(Herufi('é')))
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
| `a.pata(i)`       | `Chaguo<T>`    | Read an element safely by index                         |
| `a.badilisha(i,v)`| `Tupu`         | Replace an existing element in place                    |
| `a.ramani(f)`     | `Orodha<T>`    | Map each element through a named function                |
| `a.chuja(f)`      | `Orodha<T>`    | Keep elements for which a named function returns `kweli` |
| `a.hesabu(f)`     | `Namba`        | Count elements for which a named function returns `kweli`|
| `a.chunguza(f)`   | `Ukweli`       | Check whether any element passes a named predicate       |
| `a.unganisha(sep)`| `Neno`         | Join a list of strings with a separator                  |
| `a.jiunge(sep)`    | `Neno`         | Join values convertible to strings with a separator      |
| `a.kwa_neno()`     | `Orodha<Neno>` | Convert every element to a string                        |
| `a.vipande(size)`  | `Orodha<Orodha<T>>` | Split the list into chunks of at most `size` elements |
| `a.kila_na_fahirisi(f)` | `Tupu`   | Call a named function with each element and its index    |
| `a.tupu()`         | `Ukweli`       | Whether the list has no elements                         |
| `a.kwanza()` / `a.mwisho()` | `Chaguo<T>` | The first / last element, or `Hamna` when empty  |
| `a.ina(x)`         | `Ukweli`       | Whether some element `==` `x`                            |
| `a.tafuta(x)`      | `Chaguo<Namba>`| Index of the first element `==` `x`, or `Hamna`          |
| `a.kata(i, j)`     | `Orodha<T>`    | Elements `i` to `j` (exclusive; `j` optional; both clamped) |
| `a.geuza()`        | `Orodha<T>`    | The elements in reverse order                            |
| `a.panga()`        | `Orodha<T>`    | Sorted ascending, stable: numbers (NaN last), text, characters or truth values — all of one kind, or an error |
| `a.panga_kwa(f)`   | `Orodha<T>`    | Sorted (stable) by the key the named function gives each element (each key computed once) |
| `a.kubwa()` / `a.ndogo()` | `Chaguo<T>` | The largest / smallest element in `panga`'s order (the first, among equals), or `Hamna` when empty |
| `a.jumla()`        | `Namba`        | The sum of a list of numbers, added left to right        |
| `a.kipekee()`      | `Orodha<T>`    | The elements without repeats, first occurrences in order (repeats as a `Seti` counts them: `0` and `-0` differ) |
| `a.ongeza_zote(b)` | `Tupu`         | Append every element of `b` (mutates in place)           |
| `a.futa_zote()`    | `Tupu`         | Remove every element (mutates in place; copies keep theirs) |
| `a.ingiza(i, v)`  | `Tupu`         | Replace the element at index `i` with `v` (mutates in place); panics if `i` is out of bounds — does **not** grow the list. `a[i] = v` desugars to this call. |
| `a[i]`            | `T`            | Element; out of range is a runtime error (not a method — see below) |
| `a[i]?`           | `T`            | Element; out of range returns the `KosaMipaka` error from the `kazi` |

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

kazi kwa_mstari(row: Orodha<Namba>) -> Neno {
  rejesha row.kwa_neno().jiunge(" ")
}
weka mistari = a.vipande(2).ramani("kwa_mstari").jiunge("\n")

[3, 1, 2].panga()            # [1, 2, 3]
[3, 1, 3].kipekee()          # [3, 1]
[4, 9, 2].kubwa()            # Chaguo(Kuna(9))
[1.5, 2, 3].jumla()          # 6.5
```

On an `Orodha<Namba>` these methods work on the list's compact numeric storage directly, and
a list result (`panga`, `kata`, `geuza`, `kipekee`) stays one: sorting integers is a radix sort,
`jumla` of integers that cannot overflow 2^53 adds them as machine integers (the same result,
several times faster than a loop), and searching an integer list for a value it cannot hold
answers at once.

### Kufikia Kipengele (Indexing)

```asili
weka a = orodha(5, 10, 15)
weka x = a[0]     # 5
weka y = a[2]     # 15
weka z = a[7]     # kosa: fahirisi nje ya mipaka: 7 (urefu 3)
weka w = a[7]?    # returns Tokeo(Kosa(KosaMipaka)) from the enclosing kazi instead
weka v = a.pata(7).angu(0)   # 0 — Chaguo-based, never fails

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
| `m.ondoa(k)`       | `Chaguo<V>`    | Remove key `k`, returning its value; `Hamna` if it was missing (mutates in place) |
| `m.thamani()`      | `Orodha<V>`    | All values (in the same order as `funguo()`)         |
| `m.vipengele()`    | `Orodha<Jozi<K,V>>` | All entries as key–value pairs                  |
| `m.futa_zote()`    | `Tupu`         | Remove every entry (mutates in place)                |

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

## Seti — Set Methods

| Njia                 | Matokeo     | Maelezo                                         |
|----------------------|-------------|-------------------------------------------------|
| `s.ongeza(x)`        | `Tupu`      | Add `x` (mutates in place)                      |
| `s.ondoa(x)`         | `Ukweli`    | Remove `x`; whether it was there                |
| `s.ina(x)`           | `Ukweli`    | Whether `x` is in the set                       |
| `s.urefu()`          | `Namba`     | Number of elements                              |
| `s.orodha()`         | `Orodha<T>` | The elements as a list (order not guaranteed)   |
| `s.clona()`          | `Seti<T>`   | Copy                                            |
| `s.muungano(t)`      | `Seti<T>`   | Elements in `s` or `t` (union)                  |
| `s.makutano(t)`      | `Seti<T>`   | Elements in both (intersection)                 |
| `s.tofauti(t)`       | `Seti<T>`   | Elements of `s` not in `t` (difference)         |
| `s.ni_sehemu_ya(t)`  | `Ukweli`    | Whether every element of `s` is in `t`          |

```asili
weka a = seti(1, 2, 3)
weka b = seti(2, 3, 4)
a.makutano(b).orodha().panga()   # [2, 3]
seti(2).ni_sehemu_ya(a)          # kweli
```

---

## Wakati — Time Methods

| Njia          | Matokeo | Maelezo                              |
|---------------|---------|--------------------------------------|
| `t.sekunde()` | `Namba` | Seconds since the Unix epoch         |

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
