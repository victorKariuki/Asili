# Umbo (struct) — REPL help

`umbo` defines a named structure with typed fields.

## Kuunda Aina (Define)

```
> umbo Nukta { x: Namba, y: Namba }
```

## Kuanzisha (Instantiate)

```
> weka p = Nukta { x: 3.0, y: 4.0 }
```

## Ufikiaji wa Mashamba (Field Access)

```
> p.x
Namba(3.0)
> p.y
Namba(4.0)
```

## Kubadilisha Mashamba (Field Update)

```
> p.x = 10.0
> p.x
Namba(10.0)
```

## Mbinu za Kibinafsi (Methods via shughuli ya)

```
> umbo Duara { r: Namba }
> shughuli ya Duara {
    kazi eneo(nafsi) -> Namba { rejesha PI * nafsi.r * nafsi.r }
  }
> weka d = Duara { r: 5.0 }
> d.eneo()
Namba(78.539…)
```

## Kuvunjua kwa Linganisha (Struct Destructuring)

```
> umbo Pika { x: Namba, y: Namba }
> weka p = Pika { x: 1.0, y: 2.0 }
> linganisha p {
    Pika { x, y } => { chapisha(x kama Neno) }
    _ => {}
  }
1
```

## Mfano Kamili

```
> umbo Mtu { jina: Neno, umri: Namba }
> shughuli ya Mtu {
    kazi salamu(nafsi) -> Neno {
      rejesha "Habari, " + nafsi.jina + "!"
    }
  }
> weka m = Mtu { jina: "Amara", umri: 30 }
> m.salamu()
Neno("Habari, Amara!")
> m.umri
Namba(30.0)
```

## Ona pia

- `?aina` — aina za msingi
- `?makosa` — Chaguo na Tokeo
