# Mifano (Examples)

Real programs from the `examples/` directory, annotated to show the patterns in use.

---

## 1. Hesabu Rahisi — `examples/asi_sample`

The simplest runnable project: imports, function calls, print.

```asili
leta mfumo
leta hisabati
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka j = jumla(2, 3)
  chapisha("jumla(2, 3) = " + (j kama Neno))

  weka d = duara(4)
  chapisha("duara(4) = " + (d kama Neno))
}
```

**Mifumo inayoonyeshwa:** `leta`, `weka`, builtin math, `kama Neno` cast for printing.

---

## 2. Muundo na Shughuli — `examples/phase1_modules`

Structs with methods and field access.

```asili
leta msingi
leta matumizi

umbo Jozi {
  x: Namba
  y: Namba
}

shughuli ya Jozi {
  kazi jumla_kuratibu(hii: Jozi) -> Namba {
    rejesha hii.x + hii.y
  }

  kazi umbali(hii: Jozi) -> Namba {
    weka dx = hii.x
    weka dy = hii.y
    rejesha mizizi(dx * dx + dy * dy)
  }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka p = Jozi { x: 3.0, y: 4.0 }
  chapisha("Jumla: " + (p.jumla_kuratibu() kama Neno))   # 7.0
  chapisha("Umbali: " + (p.umbali() kama Neno))           # 5.0
}
```

**Mifumo inayoonyeshwa:** `umbo`, `shughuli ya`, method dispatch, field access, `mizizi` from msingi prelude.

---

## 3. Udhibiti wa Mtiririko — `examples/control_structures`

Every control flow construct in one file.

```asili
# Range loop
kazi jumla_hadi(n: Namba) -> Namba {
  weka jumla = 0
  kwa i kutoka 0 hadi n {
    jumla += i
  }
  rejesha jumla
}

# while with break
kazi tafuta_kwanza(orodha_: Orodha<Namba>, lengo: Namba) -> Namba {
  weka i = 0
  wakati i < orodha_.urefu() {
    weka x = orodha_[i]?
    ikiwa x == lengo { rejesha i }
    i += 1
  }
  rejesha -1
}

# match with default arm
kazi daraja(alama: Namba) -> Neno {
  linganisha alama {
    100 => { rejesha "Kamili" }
    _   => {
      ikiwa alama >= 70 { rejesha "Nzuri" }
      rejesha "Jaribu tena"
    }
  }
}

# labeled break out of nested loops
kazi tafuta_matrix(grid: Orodha<Orodha<Namba>>, lengo: Namba) -> Jozi<Namba,Namba> {
  weka safu = 0
  lebo 'nje: wakati safu < grid.urefu() {
    weka safu_data = grid[safu]?
    weka nguzo = 0
    wakati nguzo < safu_data.urefu() {
      ikiwa safu_data[nguzo]? == lengo { vunja 'nje }
      nguzo += 1
    }
    safu += 1
  }
  rejesha jozi(safu, 0)
}
```

**Mifumo inayoonyeshwa:** `kwa kutoka hadi`, `wakati`, `lebo`, labeled `vunja`, `linganisha` with `_`, `+=`.

---

## 4. Miundo ya Data — `examples/data_structures`

Lists, dictionaries, pairs, and structs together.

```asili
# Building a frequency map
kazi hesabu_maneno(maandishi: Neno) -> Kamusi<Neno, Namba> {
  weka maneno = maandishi.gawanya(" ")
  weka idadi = kamusi_tupu()

  kwa neno katika maneno {
    weka sasa = idadi.pata(neno).angu(0)
    idadi.ingiza(neno, sasa + 1)
  }
  rejesha idadi
}

# Sorting with index tracking (manual)
kazi pata_kubwa(namba: Orodha<Namba>) -> Jozi<Namba, Namba> {
  weka kubwa = namba[0]?
  weka nafasi = 0
  weka i = 1
  wakati i < namba.urefu() {
    weka x = namba[i]?
    ikiwa x > kubwa {
      kubwa = x
      nafasi = i
    }
    i += 1
  }
  rejesha jozi(kubwa, nafasi)
}
```

**Mifumo inayoonyeshwa:** `gawanya` → iterate, `kamusi_tupu` + `ingiza`, `.angu(default)`, nested `jozi`.

---

## 5. Kushughulikia Makosa — Error Handling

Safe division, chained fallbacks, index propagation.

```asili
leta hisabati
leta matumizi

kazi gawanya_salama(a: Namba, b: Namba) -> Neno {
  weka matokeo = gawio(a, b)
  linganisha matokeo {
    Hamna => { rejesha "Kosa: gawanya na sifuri" }
    _     => { rejesha "Jibu: " + (jaribu gawio(a, b) kama Neno) }
  }
}

# Chained .angu() fallbacks
kazi pata_usanidi(m: Kamusi<Neno, Neno>) -> Neno {
  weka mwenyeji  = m.pata("mwenyeji").angu("localhost")
  weka bandari   = m.pata("bandari").angu("8080")
  rejesha mwenyeji + ":" + bandari
}

# Propagate index errors up the call stack
kazi kipengele_cha_tatu(a: Orodha<Namba>) -> Namba {
  rejesha a[2]?   # propagates if list has < 3 elements
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha(gawanya_salama(10, 2))    # Jibu: 5
  chapisha(gawanya_salama(10, 0))    # Kosa: gawanya na sifuri

  weka usanidi = kamusi_tupu()
  usanidi.ingiza("bandari", "3000")
  chapisha(pata_usanidi(usanidi))    # localhost:3000
}
```

---

## 6. A\* Algorithm — `examples/astar`

A full algorithm showing all features together: nested loops, maps, list indexing, labeled break, method chaining.

Key patterns:

```asili
# Map key from tuple coords
weka key = (px kama Neno) + "," + (py kama Neno)

# Dict with default
weka g = gcosts.pata(key).angu(1000000.0)

# Conditional update
ikiwa newg < oldg {
  gcosts[nkey.clona()] = newg
  parents[nkey] = ckey
  openlist.ongeza(jozi(nx, ny))
}

# Optional index access
weka pos = openlist[i]?
weka px = pos.kwanza()
```

Full source: [examples/astar/src/kuu.as](../../examples/astar/src/kuu.as)

---

## 7. Aina Zote — `examples/all_types`

A reference showing every type in action:

```asili
# Integer overflow is a Chaguo
weka b8 = jaribu (1000 kama Biti8)   # Hamna — 1000 overflows Biti8

# Herufi conversion
weka h = 65 kama Herufi     # 'A'
weka s = h kama Neno        # "A"

# Struct cast to Neno
umbo Pika { x: Namba, y: Neno }
weka p = Pika { x: 1, y: "a" }
weka repr = p kama Neno     # "Pika { x: 1, y: \"a\" }"

# Orodha of structs
weka a = orodha()
a.ongeza(Pika { x: 1, y: "kwanza" })
a.ongeza(Pika { x: 2, y: "pili" })
chapisha(a.urefu() kama Neno)   # 2
```

Full source: [examples/all_types/src/kuu.as](../../examples/all_types/src/kuu.as)

---

## 8. Pembejeo ya Mtumiaji (Interactive Input)

```asili
leta matumizi
leta majira

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka jina = omba("Jina lako: ")
  chapisha("Habari, " + jina + "!")

  weka wakati_sasa = sasa()
  chapisha("Wakati wa sasa: " + umbiza(wakati_sasa, "%Y-%m-%d %H:%M:%S"))
}
```

**Mifumo inayoonyeshwa:** `omba` (readline), `sasa`, `umbiza` with format string.

---

## Mwongozo wa Mifano

| Mfano                  | Msimbo                                    | Mifumo Makuu                     |
|------------------------|-------------------------------------------|----------------------------------|
| `asi_sample`           | [kuu.as](../../examples/asi_sample/src/kuu.as)       | Basics, math module              |
| `phase1_modules`       | [kuu.as](../../examples/phase1_modules/src/kuu.as)   | Structs, methods                 |
| `phase1_interactive`   | [kuu.as](../../examples/phase1_interactive/src/kuu.as)| User input, runtime info        |
| `control_structures`   | [kuu.as](../../examples/control_structures/src/kuu.as)| All loop and branch forms       |
| `data_structures`      | [kuu.as](../../examples/data_structures/src/kuu.as)  | Orodha, Kamusi, Jozi, Struct    |
| `binary_ops`           | [kuu.as](../../examples/binary_ops/src/kuu.as)       | All operators, precedence        |
| `unary_ops`            | [kuu.as](../../examples/unary_ops/src/kuu.as)        | Unary ops, NaN, Inf              |
| `all_types`            | [kuu.as](../../examples/all_types/src/kuu.as)        | Every type and cast combination  |
| `astar`                | [kuu.as](../../examples/astar/src/kuu.as)            | Real algorithm, all features     |
