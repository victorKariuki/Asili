# Udhibiti (control flow) — REPL help

REPL inakubali amri moja kwa wakati. Mzunguko na masharti yanafanya kazi ndani ya mstari mmoja.

## Masharti

```
> ikiwa kweli { chapisha("ndio") }
ndio

> weka x = 5
> ikiwa x > 3 { chapisha("kubwa") } vinginevyo { chapisha("ndogo") }
kubwa
```

## Linganisha

`linganisha` is a statement, not a value-producing expression — it cannot be assigned
(`weka r = linganisha ... ` fails to parse) and a bare value inside an arm is just a discarded
expression statement, not a returned result. Call `chapisha` (or another side-effecting
statement) inside each arm instead:

```
> weka n = 2
> linganisha n { 1 => { chapisha("moja") } 2 => { chapisha("mbili") } _ => { chapisha("nyingine") } }
mbili
```

## Mzunguko

```
> weka jumla = 0
> kwa i kutoka 1 hadi 6 { jumla = jumla + i }
> jumla
Namba(15.0)

> weka i = 0
> wakati i < 3 { chapisha(i kama Neno)  i = i + 1 }
0
1
2
```

## Kudhibiti Mzunguko

```
> kwa i kutoka 0 hadi 10 { ikiwa i == 3 { vunja } chapisha(i kama Neno) }
0
1
2

> kwa i kutoka 0 hadi 6 { ikiwa i % 2 != 0 { endelea } chapisha(i kama Neno) }
0
2
4
```

## Ona pia

- `?kazi` — msaada wa kufafanua kazi
- `?orodha` — msaada wa orodha
- For more on `linganisha`, see `docs/language/03-udhibiti.md`.
