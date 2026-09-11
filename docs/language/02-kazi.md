# Kazi (Functions)

## Kufafanua Kazi (Defining Functions)

```asili
kazi salamu(jina: Neno) -> Neno {
  rejesha "Habari, " + jina
}
```

- `kazi` — keyword introducing a function
- Parameters are `name: Type` pairs separated by commas
- `-> Type` declares the return type (omit or use `-> Tupu` for void)
- `rejesha` returns a value

## Kuita Kazi (Calling Functions)

```asili
weka jibu = salamu("Amara")
chapisha(jibu)    # Habari, Amara
```

## Kazi bila Matokeo (Void Functions)

```asili
kazi chapisha_kisanduku(ujumbe: Neno) -> Tupu {
  chapisha("┌─────────────┐")
  chapisha("│ " + ujumbe + " │")
  chapisha("└─────────────┘")
}
```

## Vigezo Vingi (Multiple Parameters)

```asili
kazi ongeza(a: Namba, b: Namba) -> Namba {
  rejesha a + b
}

kazi umbiza_jibu(a: Namba, b: Namba, neno: Neno) -> Neno {
  rejesha ((a + b) kama Neno) + " " + neno
}
```

`kama` binds tighter than `+` (see [09-waendeshaji.md](09-waendeshaji.md)), so casting the
*sum* requires explicit parens — `a + b kama Neno` would parse as `a + (b kama Neno)`.

## Kujumuisha Moduli (Importing Modules)

Functions from the standard library require an explicit import:

```asili
leta matumizi     # chapisha, omba, onyo, makosa
leta hisabati     # jumla, gawio, mzizi, sakafu, nk.
leta mfumo        # vigezo, pata_env, toka
```

## Kazi Zinazozunguka (Recursive Functions)

```asili
kazi sababu(n: Namba) -> Namba {
  ikiwa n <= 1 { rejesha 1 }
  rejesha n * sababu(n - 1)
}
```

## Kazi Kuu (Entry Point)

Every runnable Asili project has a `kuu` function:

```asili
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("Habari Dunia!")
}
```

`hoja` receives command-line arguments passed after the entrypoint. The CLI does **not**
strip a `--` separator — anything after `--tenda` is passed through literally, `--` included:

```bash
pata jenga --tenda hello world
```

## Kazi za Muundo (Struct Methods)

Use `shughuli ya TypeName` to attach methods:

```asili
umbo Mduara { r: Namba }

shughuli ya Mduara {
  kazi eneo(self: Mduara) -> Namba {
    rejesha 3.14159 * self.r * self.r
  }
}

weka m = Mduara { r: 5 }
weka e = m.eneo()
```

## Vigezo Visivyobadilika Ndani ya Kazi (Immutable Parameters)

Parameters are immutable by default. Shadow them with `weka` if you need mutation:

```asili
kazi piga_mara_mbili(n: Namba) -> Namba {
  weka matokeo = n * 2
  rejesha matokeo
}
```
