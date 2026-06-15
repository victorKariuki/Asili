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

```
> weka n = 2
> linganisha n { 1 => { "moja" } 2 => { "mbili" } _ => { "nyingine" } }
Neno("mbili")
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
- `?linganisha` — (tazama 03-udhibiti.md katika docs/language/)
