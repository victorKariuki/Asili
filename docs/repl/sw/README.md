# Hati za Mada za REPL

Tumia `?mada` ndani ya REPL kuonyesha msaada (mf. `?hisabati`, `?orodha`).

Mada zote 11 zipo leo (`?mada` kwa mada isiyokuwepo itashindwa tu kupata faili —
hakuna nafasi tupu ya akiba).

| Mada          | Faili             | Maelezo                          |
|---------------|-------------------|----------------------------------|
| `aina`        | aina.md           | Aina, ubadilishaji wa aina, maadili maalum |
| `hisabati`    | hisabati.md       | Kazi za moduli ya hisabati       |
| `jozi`        | jozi.md           | Kuunda na njia za jozi           |
| `kamusi`      | kamusi.md         | Kuunda na njia za kamusi         |
| `kazi`        | kazi.md           | Semi/kauli kwenye REPL (kazi haziwezi kutamkwa moja kwa moja — tazama chini) |
| `makosa`      | makosa.md         | Chaguo, Tokeo, ushughulikiaji wa makosa |
| `neno`        | neno.md           | Njia na shughuli za neno         |
| `orodha`      | orodha.md         | Kuunda na njia za orodha         |
| `udhibiti`    | udhibiti.md       | Udhibiti wa mtiririko kwenye REPL |
| `umbo`        | umbo.md           | Umbo (linaundwa kwenye faili, si moja kwa moja — tazama chini) |
| `waendeshaji` | waendeshaji.md    | Waendeshaji, kipaumbele, biti    |

## Kuanza REPL

```bash
pata repl
```

Matokeo:
```
Asili REPL. Andika 'toka' au 'exit' kuondoka. ?mada = msaada (mf. ?hisabati).
>
```

## Amri za REPL

| Amri              | Maelezo                              |
|-------------------|----------------------------------------|
| `?mada`           | Onyesha msaada wa mada (mf. `?orodha`) |
| `?lugha en/sw`    | Badilisha lugha ya hati za msaada (Kiingereza/Kiswahili) |
| `toka` / `exit`   | Funga REPL                            |
| Usemi wowote      | Tathmini na chapisha matokeo          |
| Kauli yoyote      | Tekeleza (mf. `weka x = 5`)           |

## Mfano wa Kikao

```
> weka x = 10
> weka y = 25
> x + y
Namba(35.0)
> "Jibu ni: " + ((x + y) kama Neno)
Neno("Jibu ni: 35")
> ?hisabati
(inaonyesha hisabati.md)
> toka
```

## Vipengele

- Hali huendelea kutoka mstari mmoja hadi mwingine (vigezo vinabaki vimewekwa)
- Semi tupu hurejeshwa na kuchapishwa kiotomatiki
- `?mada` husoma `docs/repl/{lugha}/{mada}.md` moja kwa moja pale inapohitajika — hakuna
  orodha maalum, hakuna ukaguzi wa saraka, hivyo faili lisilopo hushindwa tu kufunguka
- Makosa huonyeshwa papo hapo bila kuvunja kikao
- **Vitu vingi vya ngazi ya juu haviwezi kutamkwa moja kwa moja.** Kila mstari hutathminiwa kana
  kwamba ni kauli moja ndani ya mwili wa kazi — hivyo `kazi`, `umbo`, `shughuli ya` (vizuizi vya
  utekelezaji), `jenum`, `sifa`, na `leta` vyote havichanganui kwenye kiashiria cha `>`.
  `thabiti` ndiyo ubaguzi — hufanya kazi kama `weka` (kauli ya kawaida) na hufanya kazi vizuri.
  REPL ni kwa ajili ya kutathmini semi/kauli dhidi ya vitu vilivyojengwa tayari; andika kazi/umbo/
  moduli mpya kwenye faili la `.as` na uliendeshe kwa `pata jenga --tenda`. Tazama
  [?kazi](kazi.md) na [?umbo](umbo.md).
- Mstari unaojirudia (kutathmini usemi wowote huhesabiwa) huchapisha mstari wa mwisho
  `  (undani: N)` unaoonyesha undani wa juu wa wito wa tathmini — hii huonekana baada ya karibu
  kila tathmini halisi lakini imeondolewa kwenye mifano ya hapo juu na kwenye hati nyingine za
  mada kwa urahisi wa kusoma; itarajie kuiona kwa vitendo.
