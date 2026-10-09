# Kamusi (dictionary) — REPL help

Kamusi ni hifadhi ya ufunguo-thamani. Ufunguo unaweza kuwa wa aina yoyote inayoweza kuhashiwa.

## Kuunda

```
> weka m = kamusi_tupu()
> weka m2 = {"jina": "Baraka", "umri": 30}
```

## Njia

| Expression         | Description                                     |
|--------------------|-------------------------------------------------|
| `m.ingiza(k, v)`   | Set or update key `k` to value `v`              |
| `m.pata(k)`        | The value (Chaguo: Hamna when missing)          |
| `m.pata(k).angu(d)`| The value, or `d` when the key is missing       |
| `m.ondoa(k)`       | Remove key `k`, returning its value (Chaguo)    |
| `m.thamani()`      | All values as a list                            |
| `m.vipengele()`    | All key–value pairs (`Orodha<Jozi>`)            |
| `m.futa_zote()`    | Remove every entry                              |

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
