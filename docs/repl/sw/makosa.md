# Makosa — Msaada wa REPL

Asili ina aina mbili za kushughulikia makosa: `Chaguo<T>` na `Tokeo<T,E>`.

## Chaguo — Thamani Inayoweza Kukosekana

```
> weka m = kamusi_tupu()
> m.ingiza("x", 42)
> m.pata("x")
Chaguo(Kuna(Namba(42.0)))
> m.pata("z")
Chaguo(Hamna)
> m.pata("x").angu(0)
Namba(42.0)
> m.pata("z").angu(99)
Namba(99.0)
> m.pata("x").ni_po()
Ukweli(true)
> m.pata("z").ni_tupu()
Ukweli(true)
```

## Tokeo — Operesheni Inayoweza Kushindwa

```
> gawio(10, 2)
Tokeo(Sawa(Namba(5.0)))
> gawio(10, 0)
Tokeo(Kosa(Neno("gawio kwa sifuri")))
> jaribu gawio(10, 2)
Namba(5.0)
> gawio(10, 2).ni_sawa()
Ukweli(true)
> gawio(10, 0).ni_kosa()
Ukweli(true)
> gawio(10, 0).angu(-1)
Namba(-1.0)
```

## Kuunda Tokeo na Chaguo

```
> tokeo(42)
Tokeo(Sawa(Namba(42.0)))
> kosa("Kitu kimekwenda vibaya")
Tokeo(Kosa(Neno("Kitu kimekwenda vibaya")))
> chaguo(100)
Chaguo(Kuna(Namba(100.0)))
> Hamna
Hamna
```

## Njia za Chaguo

| Njia             | Maelezo                                 |
|------------------|-----------------------------------------|
| `.angu(mbadala)` | Thamani au `mbadala` ikiwa Hamna        |
| `.ni_po()`       | `kweli` ikiwa ina thamani               |
| `.ni_tupu()`     | `kweli` ikiwa Hamna                     |
| `.hakikisha(msg)`| Toa thamani au paparika na `msg`        |

## Njia za Tokeo

| Njia             | Maelezo                                 |
|------------------|-----------------------------------------|
| `.angu(mbadala)` | Thamani Ok au `mbadala` kwenye Kosa    |
| `.ni_sawa()`     | `kweli` ikiwa Ok                        |
| `.ni_kosa()`     | `kweli` ikiwa Kosa                      |
| `.kosa()`        | Pata thamani ya kosa                    |

## Ona pia

- `?aina` — maelezo ya aina Chaguo na Tokeo
