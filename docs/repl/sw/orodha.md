# Orodha — Msaada wa REPL

Orodha ni mkusanyiko unaohesabu kwa index.

## Uundaji na njia

- `orodha(...)` — unda orodha kutoka hoja (mf. `orodha(1, 2, 3)`)
- `a.urefu()` — urefu wa orodha
- `a[0]` — kipengele kwa index (kwanza = 0); index nje ya mipaka husimamisha programu kwa kosa
- `a[0]?` — kipengele, au hurudisha kosa la `KosaMipaka` kutoka kazi ikiwa index iko nje ya mipaka
- `a.pata(i)` — `Chaguo`: `Kuna(x)` au `Hamna` ikiwa index iko nje ya mipaka
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
