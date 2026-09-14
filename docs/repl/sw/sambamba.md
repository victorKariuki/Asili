# Sambamba — Msaada wa REPL

`tenda`/`njia`/`fungo` ni vitu vya ngazi ya juu (au vinahitaji `kazi` za ngazi ya juu kuita) —
`kazi` haiwezi kutamkwa kwenye kiashiria cha REPL, hivyo mifano hii lazima iandikwe katika faili
la `.as` na iendeshwe kwa `pata jenga --tenda` (tazama `?kazi`).

Mfumo wa 1:1 nyuzi za OS halisi — `tenda` moja huanzisha uzi mmoja wa OS, si nyuzi za kijani.

## tenda / subiri_tenda

```asili
leta sambamba
leta matumizi

kazi mfanyakazi(tx: NjiaTx<Namba>, n: Namba) -> Tupu {
    jaribu (tx.tuma(n * n))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka p = njia()
    weka tx = p.kwanza()
    weka rx = p.pili()

    weka id = jaribu (tenda("mfanyakazi", tx, 5))
    weka jibu = jaribu (rx.pokea())
    chapisha(jibu kama Neno)          // "25"
    jaribu (subiri_tenda(id))
}
```

**Kumbuka muhimu:** thamani inayorejeshwa na kazi iliyoanzishwa na `tenda` **haiwezi** kupitia
`subiri_tenda` moja kwa moja — `subiri_tenda(id)` inarejesha tu `Tokeo<Tupu, Neno>` (uzi
ulikamilika au ulianguka). Tumia `njia` kutuma matokeo halisi kurudi.

`Kasha_GC<T>`/`Faili`/`Mkondo` haziwezi kupitishwa kama hoja za `tenda` — hukataliwa na `Kosa`.

## Njia

| Njia          | Maelezo                                              |
|---------------|---------------------------------------------------------|
| `njia()`      | Unda `NjiaTx<T>`/`NjiaRx<T>` mpya kama `Jozi` — haiwezi kushindwa, haina kikomo |
| `njia_na_kikomo(kikomo)` | Kama `njia()` lakini `tx.tuma(v)` **husubiri** kikomo kikijaa badala ya kukua bila mpaka |
| `tx.tuma(v)`  | Tuma; `Tokeo<Tupu, Neno>`                            |
| `rx.pokea()`  | Pokea (inasubiri); `Tokeo<T, Neno>` — `Kosa` mara zote za kutuma zinapofungwa |

## Fungo

```asili
leta sambamba

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka f = jaribu (fungo(0.0))
    f.weka(42.0)
    chapisha(f.pata() kama Neno)   // "42"
}
```

| Njia          | Maelezo                                              |
|---------------|---------------------------------------------------------|
| `fungo(v)`    | Funga `v`; `Tokeo<Fungo<T>, Neno>`                   |
| `f.pata()`    | Funga+soma+fungua — hatua moja salama                |
| `f.weka(v)`   | Funga+andika+fungua — hatua moja salama              |
| `f.funga()` / `f.fungua()` | Kufunga/kufungua wazi kwa shughuli kadhaa pamoja — `.fungua()` bila `.funga()` iliyotangulia ni kosa (paparika) |

## Ona pia

- `?kazi` — kwa nini vitu vya ngazi ya juu havifanyi kazi kwenye REPL
- `?faili` — vishikizo vingine vinavyokataliwa na `tenda`/`fungo`
