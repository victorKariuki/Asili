# Kamusi (dictionary) — REPL help

Kamusi ni hifadhi ya ufunguo-thamani. Ufunguo unaweza kuwa wa aina yoyote inayoweza kuhashiwa.

## Kuunda

```
> weka m = kamusi_tupu()
> weka m2 = {"jina": "Baraka", "umri": 30}
```

## Njia

| Usemi              | Maelezo                                         |
|--------------------|-------------------------------------------------|
| `m.ingiza(k, v)`   | Weka au sasisha ufunguo `k` na thamani `v`      |
| `m.pata(k)`        | Pata thamani (Chaguo: Hamna ikiwa haipo)        |
| `m.pata(k).angu(d)`| Pata thamani au `d` ikiwa ufunguo haupo         |

## Mfano

```
> weka m = kamusi_tupu()
> m.ingiza("mji", "Nairobi")
> m.ingiza("idadi", 5000000)
> m.pata("mji")
Chaguo(Some(Neno("Nairobi")))
> m.pata("mji") kama Neno
Neno("Nairobi")
> m.pata("nchi").angu("Haijulikani")
Neno("Haijulikani")
```

## Ufunguo wa Aina Tofauti

```
> weka m = kamusi_tupu()
> m.ingiza(1, "moja")
> m.ingiza(kweli, "ukweli")
> m.ingiza('a', "herufi a")
> m.pata(1) kama Neno
Neno("moja")
```
