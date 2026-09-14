# Aina — Msaada wa REPL

## Aina za Msingi

| Aina      | Mfano                     | Maelezo                  |
|-----------|---------------------------|--------------------------|
| `Namba`   | `3.14`, `100`, `-7`       | Namba ya desimali        |
| `Neno`    | `"Habari"`, `""`          | Mfuatano wa UTF-8        |
| `Ukweli`  | `kweli`, `si_kweli`       | Kweli au si kweli        |
| `Herufi`  | `'a'`, `'ñ'`              | Herufi moja              |
| `Tupu`    | —                         | Hakuna thamani (kitengo) |

## Aina za Namba Kamili

`Biti8`, `Biti16`, `Biti32`, `Biti64`, `uBiti8`, `uBiti16`, `uBiti32`, `uBiti64`

```
> jaribu (100 kama Biti8)
Namba(100.0)
> 1000 kama Biti8
Chaguo(Hamna)
```

**Angalizo:** usitumie `jaribu` kwenye ubadilishaji ulio nje ya wigo ukitegemea kuona `Hamna`
kikichapishwa — `jaribu` kwenye `Chaguo(Hamna)` husababisha `paparika` ya kikao badala ya
kurudisha thamani. Badilisha aina bila `jaribu` ili kukagua `Chaguo` moja kwa moja, au ifungue
wazi kwa `.angu(default)`.

## Kubadilisha Aina (kama)

```
> 65 kama Herufi
Herufi('A')
> 'A' kama Neno
Neno("A")
> kweli kama Namba
Namba(1.0)
> "42" kama Namba
Namba(42.0)
```

## Aina za Mkusanyiko

| Aina            | Kuunda                  |
|-----------------|-------------------------|
| `Orodha<T>`     | `orodha(1, 2, 3)`       |
| `Kamusi<K,V>`   | `kamusi_tupu()`         |
| `Jozi<A,B>`     | `jozi("a", 1)`          |
| `Chaguo<T>`     | `Hamna` au thamani      |
| `Tokeo<T,E>`    | `tokeo(v)` / `kosa(e)`  |

## Maadili Maalum ya Namba

```
> Ukomo
Namba(inf)
> Siyo_Namba
Namba(NaN)
> -Ukomo
Namba(-inf)
> ni_namba(Siyo_Namba)
Ukweli(false)
> ni_ukomo(Ukomo)
Ukweli(true)
```
