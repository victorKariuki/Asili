# Seti — Msaada wa REPL

Seti ni mkusanyiko wa thamani za kipekee (hakuna marudio). Daima iko katika wigo — hauitaji
`leta`.

## Kuunda

```
> weka s = seti(1, 2, 3, 2)
> s.urefu()
Namba(3.0)
```

`seti(v1, v2, ...)` hupuuza marudio moja kwa moja. `seti_tupu()` huanzisha tupu.

## Njia

| Usemi         | Maelezo                                    |
|---------------|-----------------------------------------------|
| `s.ongeza(v)` | Ongeza mwanachama (haina athari ikiwa tayari upo) |
| `s.ondoa(v)`  | Ondoa; hurejesha `Ukweli` — kweli ikiwa alikuwepo |
| `s.ina(v)`    | `Ukweli`: je, `v` yumo?                     |
| `s.urefu()`   | Idadi ya wanachama                         |
| `s.orodha()`  | Badilisha kuwa `Orodha<T>`                 |
| `s.clona()`   | Nakala                                     |

## Mfano

```
> weka s = seti(1, 2, 3, 2)
> s.ina(2)
Ukweli(true)
> s.ongeza(9)
> s.orodha()
Orodha([Namba(2.0), Namba(3.0), Namba(1.0), Namba(9.0)])
```

**Kumbuka:** `.orodha()`'s mpangilio haujabainishwa — `Seti` hutumia `HashSet` ya Rust ndani,
hivyo usitegemee mpangilio uleule kati ya matoleo au hata kati ya kuendesha mara mbili.

## Ona pia

- `?kamusi` — mkusanyiko mwingine unaotegemea `MapKey` sawa (ufunguo-thamani badala ya seti)
- `?orodha` — `.orodha()` inabadilisha kuwa aina hii
