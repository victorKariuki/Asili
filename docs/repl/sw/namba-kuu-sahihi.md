# Namba_Kuu na Namba_Sahihi — Msaada wa REPL

`Namba_Kuu` (namba kamili isiyo na kikomo) na `Namba_Sahihi` (desimali isiyo na kikomo) hutatua
tatizo `Namba` (f64) ina nalo: kikomo cha usahihi. Zote mbili zinapatikana baada ya
`leta hisabati`.

**Kumbuka:** `leta` haiwezi kuandikwa kwenye kiashiria cha REPL moja kwa moja (tazama `?kazi`).
Mifano hii lazima iandikwe katika faili la `.as` na iendeshwe kwa `pata jenga --tenda`.

## Kuunda

Hakuna sintaksia ya kihalisia — jenga kutoka `Neno` au ubadilishe kutoka `Namba`:

```asili
leta hisabati
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka kubwa = jaribu (namba_kuu_kutoka("999999999999999999999999999999999999999999999999"))
  weka moja = jaribu (namba_kuu_kutoka("1"))
  chapisha((kubwa + moja) kama Neno)

  weka rahisi = 42 kama Namba_Kuu   # bila hitilafu (upanuzi)
}
```

## Kwa nini yanahitajika: `Namba` haiwezi kuhifadhi hii kwa usahihi

```asili
leta matumizi
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha((0.1 + 0.2) kama Neno)   # "0.30000000000000004" — hitilafu ya f64

  weka a = jaribu (namba_sahihi_kutoka("0.1"))
  weka b = jaribu (namba_sahihi_kutoka("0.2"))
  chapisha((a + b) kama Neno)       # "0.3" halisi
}
```

## Hesabu Mchanganyiko

`Namba` inapochanganywa na aina hizi, hupanuka kiotomatiki bila hitilafu ili kulingana na upande
mwingine. `Namba_Kuu` iliyochanganywa na `Namba_Sahihi` hupandishwa kuwa `Namba_Sahihi`:

```asili
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka a = jaribu (namba_kuu_kutoka("2"))
  weka b = jaribu (namba_sahihi_kutoka("0.5"))
  chapisha((a + b) kama Neno)   # "2.5"
}
```

## Ubadilishaji wa Kurudi ni Hatari (Chaguo)

```asili
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka n = jaribu (namba_kuu_kutoka("12345"))
  weka c = n kama Namba   # Chaguo<Namba>, si Namba moja kwa moja
  chapisha(c.ni_po() kama Neno)   # "kweli"
}
```

## Mgawanyo wa Namba_Kuu ni Kamili, si wa Desimali

```asili
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka a = jaribu (namba_kuu_kutoka("100"))
  weka b = jaribu (namba_kuu_kutoka("3"))
  chapisha((a / b) kama Neno)   # "33", si "33.333..."
}
```

## Ona pia

- `?hisabati` — kazi nyingine za moduli hii
- `?aina` — Namba na maadili maalum kama Ukomo/Siyo_Namba
