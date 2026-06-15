# Udhibiti wa Makosa (Error Handling)

Asili uses two built-in types for safe error handling: `Chaguo<T>` (nullable value) and `Tokeo<T,E>` (result or error).

---

## Chaguo (Option)

`Chaguo<T>` is either a value or `Hamna` (nothing).

### Kuunda

```asili
weka val = m.pata("ufunguo")    # returns Chaguo<T>
weka hakuna: Chaguo<Namba> = Hamna
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

`Tokeo<T,E>` is either `Ok(value)` or `Kosa(error)`. Used for operations that may fail.

### Kuunda

```asili
# Built-in gawio returns Tokeo<Namba, Neno>
weka jibu = gawio(10, 0)    # Tokeo::Kosa("Haiwezekani kugawanya na sifuri")
weka jibu2 = gawio(10, 2)   # Tokeo::Ok(5.0)
```

### jaribu — Kutoa Thamani Salama

`jaribu` unwraps a `Tokeo` or propagates the error:

```asili
weka matokeo = jaribu gawio(10, 2)    # 5.0
# ikiwa gawio ingerudisha Kosa, jaribu ingepeleka kosa hadi kazi inayoita
```

### Mfano Kamili

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
    _ => { chapisha("Jibu: " + (jaribu gawanya(10, 2) kama Neno)) }
  }
  linganisha gawanya(10, 0) {
    Hamna => { chapisha("Kosa lilitokea") }
    _ => {}
  }
}
```

---

## Muhtasari wa Mbinu

| Hali               | Mbinu                        |
|--------------------|------------------------------|
| Thamani inaweza kukosekana | `Chaguo<T>`, angalia na `Hamna` |
| Operesheni inaweza kushindwa | `Tokeo<T,E>`, tumia `jaribu` |
| Unataka kusimamisha mara moja | `tupa "ujumbe wa kosa"` |
| Index inaweza kuwa nje ya mipaka | `a[i]?` kueneza makosa |

---

## Kosa la Haraka kwa `tupa`

```asili
kazi hakikisha_umri(umri: Namba) -> Tupu {
  ikiwa umri < 0 {
    tupa "Umri hauwezi kuwa hasi"
  }
  ikiwa umri > 150 {
    tupa "Umri si wa kawaida"
  }
}
```

`tupa` ends execution of the current function and propagates the error value to the caller.
