# Sifa — Msaada wa REPL

`sifa` (tazama pia `?umbo`) ni **kitu cha ngazi ya juu** — kama `kazi`, `umbo`, `jenum`, na
`leta`, haiwezi kutamkwa moja kwa moja kwenye kiashiria cha REPL. Kuandika
`sifa Inayoonyeshwa { kazi onyesha(self: Self) -> Neno }` kwenye `>` hakuchanganui.

Ili kufanya kazi na sifa, ziandike katika faili la `.as` na uliendeshe kwa `pata jenga --tenda`:

```asili
leta matumizi

sifa Inayoonyeshwa {
  kazi onyesha(self: Self) -> Neno
}

umbo Paka { jina: Neno }

shughuli ya Paka kwa Inayoonyeshwa {
  kazi onyesha(self: Paka) -> Neno {
    rejesha "Paka(" + self.jina + ")"
  }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka pk = Paka { jina: "Whiskers" }
  chapisha(pk.onyesha())          # Paka(Whiskers)
}
```

Sintaksia mbili sawa kwa `shughuli ya`:

```asili
shughuli ya Paka kwa Inayoonyeshwa { ... }   # neno "kwa"
shughuli ya Paka: Inayoonyeshwa { ... }      # nukta mbili
```

## Utimilifu wa Njia

Sifa hueleza njia zinazohitajika (jina, hoja, aina ya kurudisha — bila mwili). Kila `shughuli ya`
inayotaja sifa hukaguliwa dhidi ya orodha hiyo — njia inayokosekana au yenye sahihi tofauti ni
kosa la wakati wa kuchanganua, si la wakati wa kutekeleza:

```asili
sifa Inayoonyeshwa {
  kazi onyesha(self: Self) -> Neno
  kazi jina_fupi(self: Self) -> Neno
}

umbo Paka { jina: Neno }

shughuli ya Paka kwa Inayoonyeshwa {
  kazi onyesha(self: Paka) -> Neno { rejesha self.jina }
  # jina_fupi haipo hapa
}
```

Hii hutoa `[semantiki:SEM105] njia 'jina_fupi' ya sifa 'Inayoonyeshwa' haijatekelezwa kwa
'Paka'` kabla ya kutekeleza chochote.

**Kumbuka:** utimilifu unahakikiwa tu kwa `shughuli ya` halisi zilizoandikwa na mtumiaji.
Hakuna njia ya kudai "thamani yoyote inayotekeleza sifa X" kwa jumla (hakuna `dyn`/trait-object)
leo — sifa huamua tu ni njia gani inayotumika kwenye simu ya `.njia()`, si kuruhusu hoja za kazi
zilizoandikwa kwa aina ya sifa.

## Ona pia

- `?umbo` — kwa nini vitu vya ngazi ya juu havifanyi kazi kwenye REPL
- `?faili` — `Faili`/`Mkondo` hutekeleza `Inasomeka`/`Inandikika` kwa "fiat," si kupitia
  `shughuli ya` halisi (aina hizo mbili si `umbo`)
