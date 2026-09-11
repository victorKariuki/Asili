# Umbo — Msaada wa REPL

`umbo` na `shughuli ya` (kizuizi cha utekelezaji) ni **vitu vya ngazi ya juu** — kama `kazi`,
`jenum`, `sifa`, na `leta`, haviwezi kutamkwa moja kwa moja kwenye kiashiria cha REPL (tazama
[?kazi](kazi.md) kwa sababu: kila mstari wa REPL hufunikwa kama kauli moja ndani ya mwili wa
kazi, na vitu vya ngazi ya juu havifai humo). Kuandika `umbo Nukta { x: Namba, y: Namba }`
kwenye `>` hakuchanganui.

Ili kufanya kazi na umbo, liandike katika faili la `.as` na uliendeshe kwa `pata jenga --tenda`:

```asili
leta matumizi

umbo Nukta { x: Namba, y: Namba }

shughuli ya Nukta {
  kazi eneo(self: Nukta) -> Namba { rejesha self.x * self.y }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka p = Nukta { x: 3, y: 4 }
  chapisha(p.x kama Neno)          # 3
  chapisha(p.eneo() kama Neno)     # 12
}
```

Maelezo, yaliyothibitishwa dhidi ya mkusanyaji:

- Hoja ya kwanza ya njia lazima iitwe `self` hasa (ikiwa na aina ya umbo husika):
  `kazi eneo(self: Nukta) -> Namba`. Tazama [04-muundo-data.md](../../language/04-muundo-data.md).
- Umbo **hazibadilishwi** mahali pale (`p.x = 10` haichanganui) — jenga nakala mpya badala
  yake: `weka p2 = Nukta { x: 10, y: p.y }`.
- Kuvunja umbo ndani ya `linganisha` kunahitaji muundo kamili wa `uga: kigezo` — `Nukta { x: a,
  y: b } => ...`, si mkato `Nukta { x, y } => ...` (huo haichanganui: `PAR053`).

```asili
umbo Pika { x: Namba, y: Namba }

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka p = Pika { x: 1, y: 2 }
  linganisha p {
    Pika { x: a, y: b } => { chapisha(a kama Neno) }   # 1
    _ => {}
  }
}
```

## Ona pia

- `?kazi` — kwa nini vitu vya ngazi ya juu havifanyi kazi kwenye REPL
- `?aina` — aina za msingi
- `?makosa` — Chaguo na Tokeo
