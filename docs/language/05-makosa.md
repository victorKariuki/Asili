# Udhibiti wa Makosa (Error Handling)

Asili uses two built-in types for safe error handling: `Chaguo<T>` (nullable value) and `Tokeo<T,E>` (result or error).

---

## Chaguo (Option)

`Chaguo<T>` is either a value or `Hamna` (nothing).

### Kuunda

```asili
weka val = m.pata("ufunguo")    # returns Chaguo<T>
weka hakuna = Hamna             # explicit `: Chaguo<Namba>` annotation on a bare Hamna
                                 # currently fails to type-check (SEM010) — omit it
```

### Kupata Thamani Salama

```asili
# .angu(default) unwraps or returns default
weka idadi = m.pata("idadi").angu(0)

# Pattern match
linganisha val {
  Hamna => { chapisha("Hakuna") }
  _     => { chapisha("Inapatikana") }
}
```

### Kueneza Makosa kwa `?`

The `?` operator propagates `Hamna` (or an error) upward:

```asili
weka a = orodha(5, 10, 15)
weka kipengee = a[1]?     # 10, or propagates if out of bounds
```

---

## Tokeo (Result)

`Tokeo<T,E>` is either `Ok(value)` or `Err(error)`. Used for operations that may fail.

### Kuunda

A `Tokeo` binding must be consumed — via `linganisha` or `?`/`jaribu` — in the same scope it
was created in; the checker rejects an assigned-but-unused `Tokeo` (`SEM048`). Calling a Tokeo
method (`.ni_kosa()`, `.angu()`, etc.) later, on a value stored in a variable, does **not**
count as consuming it — only `linganisha`/`?`/`jaribu` satisfy the checker.

```asili
# Built-in gawio returns Tokeo<Namba, Neno>
linganisha gawio(10, 0) {
  Tokeo::Sawa(v)  => { chapisha("jibu: " + (v kama Neno)) }
  Tokeo::Kosa(e) => { chapisha("kushindwa: " + e) }   # "Haiwezekani kugawanya na sifuri"
}
weka jibu2 = jaribu gawio(10, 2)   # 5.0 — jaribu unwraps or propagates
```

### jaribu — Kutoa Thamani Salama

`jaribu` unwraps a `Tokeo` or propagates the error:

```asili
weka matokeo = jaribu gawio(10, 2)    # 5.0
# ikiwa gawio ingerudisha Kosa, jaribu ingepeleka kosa hadi kazi inayoita
```

### Mfano Kamili

Match `Tokeo::Sawa(v)`/`Tokeo::Kosa(e)` directly — this works against a `Tokeo` returned by any
function, builtin or user-defined, and binds the wrapped value/error:

```asili
leta matumizi
leta hisabati

kazi gawanya(a: Namba, b: Namba) -> Tokeo<Namba, Neno> {
  ikiwa b == 0 {
    rejesha kosa("Haiwezekani kugawanya na sifuri")
  }
  rejesha tokeo(a / b)
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  linganisha gawanya(10, 2) {
    Tokeo::Sawa(v)  => { chapisha("Jibu: " + (v kama Neno)) }
    Tokeo::Kosa(e) => { chapisha("Kosa: " + e) }
  }
  linganisha gawanya(10, 0) {
    Tokeo::Sawa(v)  => { chapisha("Jibu: " + (v kama Neno)) }
    Tokeo::Kosa(e) => { chapisha("Kosa: " + e) }
  }
}
```

Method-based checks (`.ni_kosa()`, `.kosa()`) also work, but only against the value where it's
first produced (calling them later on a value stored in a `weka` doesn't satisfy `SEM048` — see
above):

```asili
ikiwa gawanya(10, 0).ni_kosa() {
  chapisha("Kosa lilitokea")
}
```

---

## Muhtasari wa Mbinu

| Hali               | Mbinu                        |
|--------------------|------------------------------|
| Thamani inaweza kukosekana | `Chaguo<T>`, angalia na `Hamna` |
| Operesheni inaweza kushindwa | `Tokeo<T,E>`, tumia `jaribu` |
| Unataka kusimamisha mara moja | `rejesha kosa(...)` kutoka kazi inayorudisha `Tokeo` |
| Index inaweza kuwa nje ya mipaka | `a[i]?` kueneza makosa |

---

## Kutoa Kosa Haraka (Early Error Return)

There is no "throw a value" statement — `tupa <jina>` only drops a variable from scope (see
[03-udhibiti.md](03-udhibiti.md)). To exit early with an error, return `Tokeo`'s `Kosa` variant:

```asili
kazi hakikisha_umri(umri: Namba) -> Tokeo<Namba, Neno> {
  ikiwa umri < 0 {
    rejesha kosa("Umri hauwezi kuwa hasi")
  }
  ikiwa umri > 150 {
    rejesha kosa("Umri si wa kawaida")
  }
  rejesha tokeo(umri)
}
```
