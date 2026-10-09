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
- `a.ongeza_zote(b)` / `a.futa_zote()` — ongeza vipengele vyote vya `b` / ondoa vipengele vyote
- `a.panga()` — orodha iliyopangwa kwa kupanda (Namba, Neno, Herufi au Ukweli za aina moja)
- `a.panga_kwa("f")` — panga kwa ufunguo ambao kazi `f` inautoa kwa kila kipengele
- `a.geuza()` — vipengele kwa mpangilio wa nyuma
- `a.kata(i, j)` — vipengele kutoka `i` hadi kabla ya `j`
- `a.ina(x)` / `a.tafuta(x)` — je, kipengele fulani ni sawa na `x` / index ya cha kwanza (Chaguo)
- `a.kubwa()` / `a.ndogo()` — kipengele kikubwa / kidogo zaidi (Chaguo)
- `a.jumla()` — jumla ya Namba zote
- `a.kipekee()` — vipengele bila marudio, kwa mpangilio wa kuonekana kwanza
- `a.kwanza()` / `a.mwisho()` / `a.tupu()` — kipengele cha kwanza / cha mwisho (Chaguo), je, ni tupu

## Mfano

```
> weka a = orodha(10, 20, 30)
> a[0]
Tokeo(Sawa(Namba(10.0)))
> a.urefu()
Namba(3.0)
```
