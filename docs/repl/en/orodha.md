# Orodha (list) — REPL help

Orodha ni mkusanyiko unaohesabu kwa index.

## Uundaji na njia

- `orodha(...)` — unda orodha kutoka hoja (mf. `orodha(1, 2, 3)`)
- `a.urefu()` — urefu wa orodha
- `a[0]` — element at index (first = 0); an out-of-range index stops the program with an error
- `a[0]?` — the element, or returns the `KosaMipaka` error from the `kazi` when out of range
- `a.pata(i)` — `Chaguo`: `Kuna(x)`, or `Hamna` when out of range
- `a.ongeza(x)` — ongeza kipengele mwishoni
- `a.ondoa(i)` — ondoa kipengele kwa index; rejesha Chaguo

## Mfano

```
> weka a = orodha(10, 20, 30)
> a[0]
Namba(10.0)
> a.urefu()
Namba(3.0)
```
