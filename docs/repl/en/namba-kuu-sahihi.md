# Namba_Kuu and Namba_Sahihi (arbitrary precision) — REPL help

`Namba_Kuu` (unbounded integer) and `Namba_Sahihi` (unbounded decimal) solve the problem
`Namba` (f64) has: limited precision. Both are available after `leta hisabati`.

**Note:** `leta` cannot be typed directly at the REPL prompt (see `?kazi`). These examples must
be written in a `.as` file and run with `pata jenga --tenda`.

## Creating

No literal syntax — build from a `Neno` or cast from `Namba`:

```asili
leta hisabati
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka kubwa = jaribu (namba_kuu_kutoka("999999999999999999999999999999999999999999999999"))
  weka moja = jaribu (namba_kuu_kutoka("1"))
  chapisha((kubwa + moja) kama Neno)

  weka rahisi = 42 kama Namba_Kuu   # infallible (widening)
}
```

## Why these exist: `Namba` can't hold this precisely

```asili
leta matumizi
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha((0.1 + 0.2) kama Neno)   # "0.30000000000000004" — f64 rounding

  weka a = jaribu (namba_sahihi_kutoka("0.1"))
  weka b = jaribu (namba_sahihi_kutoka("0.2"))
  chapisha((a + b) kama Neno)       # exactly "0.3"
}
```

## Mixed Arithmetic

`Namba` mixed with either of these widens infallibly to match the other side. `Namba_Kuu`
mixed with `Namba_Sahihi` promotes to `Namba_Sahihi`:

```asili
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka a = jaribu (namba_kuu_kutoka("2"))
  weka b = jaribu (namba_sahihi_kutoka("0.5"))
  chapisha((a + b) kama Neno)   # "2.5"
}
```

## The Reverse Cast Is Fallible (Chaguo)

```asili
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka n = jaribu (namba_kuu_kutoka("12345"))
  weka c = n kama Namba   # Chaguo<Namba>, not a plain Namba
  chapisha(c.ni_po() kama Neno)   # "kweli"
}
```

## Namba_Kuu Division Is Exact, Not Decimal

```asili
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka a = jaribu (namba_kuu_kutoka("100"))
  weka b = jaribu (namba_kuu_kutoka("3"))
  chapisha((a / b) kama Neno)   # "33", not "33.333..."
}
```

## See also

- `?hisabati` — other functions in this module
- `?aina` — `Namba` and special values like `Ukomo`/`Siyo_Namba`
