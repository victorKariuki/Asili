# Orodha — Msaada wa REPL

Orodha ni mkusanyiko unaohesabu kwa index.

## Uundaji na njia

- `orodha(...)` — unda orodha kutoka hoja (mf. `orodha(1, 2, 3)`)
- `a.urefu()` — urefu wa orodha
- `a[0]` — kipengele kwa index (kwanza = 0); rejesha `Tokeo` (bounds-checked)
- `a.ongeza(x)` — ongeza kipengele mwishoni
- `a.ondoa(i)` — ondoa kipengele kwa index; rejesha Chaguo

## Mfano

```
> weka a = orodha(10, 20, 30)
> a[0]
Tokeo(Sawa(Namba(10.0)))
> a.urefu()
Namba(3.0)
```
