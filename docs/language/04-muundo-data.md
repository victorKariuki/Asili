# Muundo wa Data (Data Structures)

## Orodha (List)

A dynamic, ordered, zero-indexed collection.

### Kuunda (Creation)

```asili
weka tupu = orodha()                  # empty list
weka namba = orodha(10, 20, 30)       # list of three numbers
weka maneno = orodha("a", "b", "c")   # list of strings
weka mseto = [1, 2, 3]                # list literal syntax
```

### Njia (Methods)

| Usemi              | Maelezo                                          |
|--------------------|--------------------------------------------------|
| `a.urefu()`        | Number of elements                               |
| `a[i]?`           | Element at index `i`; propagates if out of bounds|
| `a.ongeza(x)`      | Append `x` to end                                |
| `a.ondoa(i)`       | Remove element at index `i`; returns `Chaguo<T>` |
| `a.kila_mmoja(cb)` | Iterate with callback (no-op if no callback)     |

```asili
weka a = orodha(5, 10, 15)
chapisha(a.urefu() kama Neno)    # 3
chapisha(a[0]? kama Neno)        # 5
chapisha(a[2]? kama Neno)        # 15

a.ongeza(20)
chapisha(a.urefu() kama Neno)    # 4

weka kipengee = a.ondoa(1)       # removes 10, returns Chaguo(10)
```

### Kutumia Orodha (Iterating)

```asili
weka jumla = 0
kwa x katika orodha(1, 2, 3, 4, 5) {
  jumla = jumla + x
}
chapisha(jumla kama Neno)   # 15
```

---

## Kamusi (Dictionary / Map)

A key-value store. Keys can be any hashable type.

### Kuunda (Creation)

```asili
weka m = kamusi_tupu()              # empty dictionary
weka m2 = {"jina": "Amara", "mji": "Nairobi"}   # literal syntax
```

**Note:** `{...}` map literals (like `[...]` list literals) require all values to share one
type — `{"jina": "Amara", "umri": 30}` mixes `Neno` and `Namba` values and fails to compile
(`SEM100`). Use `kamusi_tupu()` + `.ingiza(...)` calls to build a dict with mixed value types.

### Njia (Methods)

| Usemi             | Maelezo                              |
|-------------------|--------------------------------------|
| `m.ingiza(k, v)`  | Insert or update key `k` with value `v` |
| `m.pata(k)`       | Returns `Chaguo<V>` (Hamna if missing) |
| `m.pata(k).angu(default)` | Returns value or `default`   |

```asili
weka m = kamusi_tupu()
m.ingiza("mji", "Nairobi")
m.ingiza("idadi", 5000000)

weka mji = m.pata("mji").angu("")           # "Nairobi" — kama Neno does NOT unwrap
                                             # a Chaguo<Neno>; use .angu() instead
weka idadi = m.pata("idadi").angu(0)        # 5000000
weka nchi = m.pata("nchi").angu("Haijulikani")  # "Haijulikani"
```

---

## Jozi (Pair / Tuple)

An immutable pair of two values (can be different types).

### Kuunda (Creation)

```asili
weka p = jozi(3, "nchi")
weka kuratibu = jozi(10.5, -4.2)
```

### Kupata Thamani (Accessing)

```asili
weka kwanza = p.kwanza()    # 3
weka pili = p.pili()        # "nchi"
```

Nested pairs:

```asili
weka nested = jozi(jozi(1, 2), "tatu")
weka ndani = nested.kwanza()
weka a = ndani.kwanza() kama Namba    # 1
weka b = ndani.pili() kama Namba      # 2
```

---

## Seti (Set)

Mkusanyiko wa thamani za kipekee (hakuna marudio). Daima iko katika wigo (kupitia `msingi`,
hauitaji `leta`).

### Kuunda (Creation)

```asili
weka s = seti(1, 2, 3, 2, 1)   # marudio yanapuuzwa: idadi 3
weka tupu2 = seti_tupu()
```

### Njia (Methods)

```asili
weka s = seti_tupu()
s.ongeza(1)                # ongeza mwanachama
weka ipo = s.ina(1)         # Ukweli: kweli
weka ilitolewa = s.ondoa(1) # Ukweli: kweli ikiwa ilikuwepo
weka idadi_w = s.urefu()    # Namba
weka orodha_w = s.orodha()  # badilisha kuwa Orodha<T>
weka nakala = s.clona()
```

**Mpangilio wa uorodheshaji haujabainishwa** — `Seti` hutumia `HashSet` ya Rust ndani, hivyo
`.orodha()` haihakikishi mpangilio uleule kati ya matoleo.

---

## Umbo (Struct)

User-defined record type with named fields.

### Kufafanua (Definition)

```asili
umbo Mtu {
  jina: Neno
  umri: Namba
  hai: Ukweli
}
```

### Kuunda Mfano (Instantiation)

```asili
weka m = Mtu { jina: "Baraka", umri: 30, hai: kweli }
```

### Kupata Vipande (Field Access)

```asili
chapisha(m.jina)             # Baraka
chapisha(m.umri kama Neno)   # 30
```

### Kuunda Upya (Rebuilding)

Asili structs are not mutated in place — create a new one:

```asili
weka m2 = Mtu { jina: m.jina, umri: m.umri + 1, hai: m.hai }
```

### Shughuli za Umbo (Methods)

The method's first parameter must be named `self` (typed with the struct it belongs to):

```asili
shughuli ya Mtu {
  kazi salamu(self: Mtu) -> Neno {
    rejesha "Habari, " + self.jina
  }
}

weka m = Mtu { jina: "Zawadi", umri: 25, hai: kweli }
chapisha(m.salamu())    # Habari, Zawadi
```

---

## Jenum (Enums)

Named variants with optional data.

```asili
jenum Rangi {
  Nyekundu
  Kijani
  Bluu
}

weka r = Rangi::Nyekundu

linganisha r {
  Rangi::Nyekundu => { chapisha("Moto") }
  Rangi::Kijani   => { chapisha("Asili") }
  Rangi::Bluu     => { chapisha("Baridi") }
  _               => { chapisha("Nyingine") }
}
```
