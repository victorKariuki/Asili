# Kazi — Msaada wa REPL

Kila mstari wa REPL hutathminiwa kana kwamba ni kauli moja ndani ya mwili wa kazi (kifuniko cha
ndani `__repl__() -> Tupu { <mstari> }`). Hii ina maana **matamko ya `kazi` — pamoja na vitu
vingine vya ngazi ya juu kama `umbo`, `shughuli ya`, `jenum`, `sifa`, `leta` — hayawezi
kuandikwa moja kwa moja kwenye kiashiria cha REPL.** Kuandika `kazi mfano(x) -> Namba {
rejesha x }` kwenye `>` hakuchanganui; hakuna hali ya mistari-mingi/kupakia-faili
inayotatua hili leo.

`thabiti` ndiyo ubaguzi pekee — hufanya kazi kama `weka` (kauli ya kawaida, si tamko la ngazi ya
moduli) na hufanya kazi vizuri kwenye REPL: `thabiti PI = 3.14` humfunga `PI` inayoweza
kutumika mara moja kwenye mstari unaofuata.

REPL ni kwa ajili ya kutathmini semi na kauli rahisi (`weka`, `ikiwa`, vitanzi, `linganisha`)
dhidi ya vitu vilivyojengwa **tayari** — si kwa kuandika kwa mkono kazi, umbo, au moduli mpya.
Ziandike katika faili la `.as` na uliendeshe kwa `pata jenga --tenda` badala yake.

## Mfano

```
> weka x = 5
> weka y = 10
> x + y
Namba(15.0)
```
