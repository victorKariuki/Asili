# Jozi (pair) — REPL help

Jozi ni muundo wa thamani mbili za aina yoyote.

## Kuunda

```
> weka p = jozi(10, "nchi")
> weka kuratibu = jozi(3.0, 4.0)
```

## Njia

| Usemi        | Matokeo  | Maelezo              |
|--------------|----------|----------------------|
| `p.kwanza()` | `A`      | Thamani ya kwanza    |
| `p.pili()`   | `B`      | Thamani ya pili      |
| `p.clona()`  | `Jozi`   | Nakala ya jozi       |

## Mfano

```
> weka p = jozi("Nairobi", 4000000)
> p.kwanza()
Neno("Nairobi")
> p.pili()
Namba(4000000.0)

> weka kuratibu = jozi(3.0, 4.0)
> (kuratibu.kwanza() kama Namba) + (kuratibu.pili() kama Namba)
Namba(7.0)
```

## Jozi zilizopachikwa

```
> weka p = jozi(jozi(1, 2), "nje")
> p.kwanza().kwanza()
Namba(1.0)
> p.kwanza().pili()
Namba(2.0)
> p.pili()
Neno("nje")
```

## Kuvunjua Mfano (Pattern Matching)

```
> weka p = jozi(5, "tano")
> linganisha p { (5, neno) => { chapisha(neno) } _ => {} }
tano
```
