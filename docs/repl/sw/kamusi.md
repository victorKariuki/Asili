# Kamusi — Msaada wa REPL

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
Chaguo(Kuna(Neno("Nairobi")))
> m.pata("mji").angu("")
Neno("Nairobi")
> m.pata("nchi").angu("Haijulikani")
Neno("Haijulikani")
```

**Kumbuka:** `kama Neno` **hai**fungui `Chaguo<Neno>` — `m.pata("mji") kama Neno` hutoa
mfuatano wa muundo wa utatuzi `Neno("Chaguo(Kuna(Neno(\"Nairobi\")))")`, si `"Nairobi"` safi.
Tumia `.angu(default)` kufungua kwanza, kama ilivyoonyeshwa juu.

## Ufunguo wa Aina Tofauti

```
> weka m = kamusi_tupu()
> m.ingiza(1, "moja")
> m.ingiza(kweli, "ukweli")
> m.ingiza('a', "herufi a")
> m.pata(1).angu("")
Neno("moja")
```
