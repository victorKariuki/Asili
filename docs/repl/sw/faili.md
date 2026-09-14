# Faili — Msaada wa REPL

`Faili` ni kishikizo cha faili — hufunga kiotomatiki kinapotoka nje ya wigo, hata bila `.funga()`
au `tupa` wazi. `Mkondo` (mkondo wa TCP) na `Kumbukumbu<T>` (kisanduku cha rundo) ni vishikizo
vinavyohusiana, vyenye tabia sawa ya njia.

**Kumbuka:** `faili_fungua`/`mkondo_unganisha` zinahitaji `leta faili`/`leta mfumo`, na `leta`
haiwezi kuandikwa moja kwa moja kwenye kiashiria cha REPL (tazama [?kazi](kazi.md) kwa sababu).
Kwa hiyo mifano hii lazima iandikwe katika faili la `.as` na iendeshwe kwa `pata jenga --tenda`,
si moja kwa moja kwenye `>`. `kumbukumbu_unda` peke yake haihitaji `leta` (iko kwenye msingi) na
inaweza kujaribiwa moja kwa moja kwenye REPL — tazama sehemu ya Kumbukumbu chini.

## Faili

```asili
leta faili

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka w = jaribu (faili_fungua("data.txt", "andika"))
  jaribu (w.andika("habari"))
  w.funga()

  weka r = jaribu (faili_fungua("data.txt", "soma"))
  chapisha(jaribu (r.soma()))   # "habari"
}
```

| Njia            | Maelezo                                    |
|-----------------|----------------------------------------------|
| `faili_fungua(njia, hali)` | Fungua; `hali` ni `"soma"`, `"andika"`, au `"ongeza"`; hurejesha `Tokeo<Faili, Neno>` |
| `w.soma()`      | Soma yote kama `Neno`; hurejesha `Tokeo<Neno, Neno>` |
| `w.andika(data)`| Andika `data`; hurejesha `Tokeo<Tupu, Neno>` |
| `w.funga()`     | Funga wazi (haina athari ikiwa tayari imefungwa) |

## Mkondo

```asili
leta mfumo

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka m = jaribu (mkondo_unganisha("127.0.0.1:8080"))
  jaribu (m.andika("GET / HTTP/1.0\r\n\r\n"))
  chapisha(jaribu (m.soma()))
  m.funga()
}
```

Njia sawa na `Faili`: `.soma()`, `.andika(data)`, `.funga()`. Pia:

| Njia | Maelezo |
|------|---------|
| `m.soma_bailisi(kikomo)` | Soma **moja tu, si mpaka EOF** — hadi `kikomo` baiti; hurejesha `Tokeo<Neno, Neno>`. Inahitajika kwa itifaki kama HTTP/1.1 keep-alive ambazo lazima zisome ujumbe mmoja kisha zisome tena kwenye muunganisho ule ule bila `.soma()` kusubiri EOF isiyokuja |

**Kusikiliza (server)** na **TLS**: `mkondo_sikiliza`, `mkondo_tumikia`, `mkondo_tumikia_http`,
`tls_sanidi` — tazama `docs/language/06-moduli.md` (sehemu ya `mfumo`) na
`docs/design/http-server-design.md`.

**JSON**: `kwa_json`/`kutoka_json` (`Value` ↔ Neno ya JSON) — tazama `docs/language/06-moduli.md` na
`docs/design/json-codec-design.md`.

## Kumbukumbu\<T\>

Hii **inaweza** kujaribiwa moja kwa moja kwenye REPL — haihitaji `leta`:

```
> weka k = kumbukumbu_unda(42.0)
> k.pata()
Namba(42.0)
```

| Njia              | Maelezo                        |
|-------------------|-----------------------------------|
| `kumbukumbu_unda(v)` | Funga `v` kwenye `Kumbukumbu<T>` mpya |
| `k.pata()`        | Soma nakala ya thamani iliyofungwa |

**Hakuna `.weka()`** — `Kumbukumbu<T>` haishirikiani kama `Kasha_GC<T>` (tazama
`docs/language/06-moduli.md`); kubadilisha kunahitaji kukabidhi jina zima upya:
`weka k = kumbukumbu_unda(thamani_mpya)`.

## Ona pia

- `?kazi` — kwa nini `leta` haiwezi kutamkwa kwenye kiashiria cha REPL
- `?makosa` — Tokeo na jinsi ya kuvitumia (`jaribu`, `linganisha`)
