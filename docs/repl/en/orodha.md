# Orodha (list) — REPL help

Orodha ni mkusanyiko unaohesabu kwa index.

## Uundaji na njia

- `orodha(...)` — unda orodha kutoka hoja (mf. `orodha(1, 2, 3)`)
- `a.urefu()` — urefu wa orodha
- `a[0]` — element at index (first = 0); an out-of-range index stops the program with an error
- `a[0]?` — the element, or returns the `KosaMipaka` error from the `kazi` when out of range
- `a.pata(i)` — `Chaguo`: `Kuna(x)`, or `Hamna` when out of range
- `a.ongeza(x)` — ongeza kipengele mwishoni
- `a.ondoa(i)` — remove the element at an index; returns a Chaguo
- `a.ongeza_zote(b)` / `a.futa_zote()` — append every element of `b` / remove every element
- `a.panga()` — sorted ascending (numbers, text, characters or truth values, all of one kind)
- `a.panga_kwa("f")` — sorted by the key the function `f` gives each element
- `a.geuza()` — the elements in reverse order
- `a.kata(i, j)` — elements `i` up to (not including) `j`
- `a.ina(x)` / `a.tafuta(x)` — whether some element `==` `x` / the index of the first (Chaguo)
- `a.kubwa()` / `a.ndogo()` — the largest / smallest element (Chaguo)
- `a.jumla()` — the sum of the numbers
- `a.kipekee()` — the elements without repeats, in first-seen order
- `a.kwanza()` / `a.mwisho()` / `a.tupu()` — first / last element (Chaguo), whether empty

## Mfano

```
> weka a = orodha(10, 20, 30)
> a[0]
Namba(10.0)
> a.urefu()
Namba(3.0)
```
