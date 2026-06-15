# Mfumo wa Aina na Uumbaji wa Hali ya Juu (Type System & Advanced Features)

## Umma — Mwonekano wa Umma (Public Visibility)

By default, functions, structs, and enums are private. Use `umma` to export them:

```asili
umma kazi salamu(jina: Neno) -> Neno {
  rejesha "Habari, " + jina
}

umma umbo Mtu {
  jina: Neno
  umri: Namba
}

umma jenum Rangi { Nyekundu; Kijani; Bluu }
```

## Thabiti za Moduli (Module-Level Constants)

```asili
thabiti PI: Namba = 3.14159265358979
thabiti JINA_PROGRAMU: Neno = "Asili"
thabiti TOLEO: Namba = 1
```

Module-level constants are evaluated at load time and are available everywhere in the file.

## Kuagiza Kwa Uchaguzi (Selective Imports)

Import only specific names from a module:

```asili
leta hisabati::{jumla, gawio, sakafu}
leta matumizi::{chapisha, omba}
```

This is equivalent to a full import but avoids name pollution when you only need a few functions.

## Vigezo vya Aina (Generic Types)

Structs and enums can be parameterized:

```asili
umbo Sanduku<T> {
  thamani: T
}

jenum Matokeo<T, E> {
  Sawa(T)
  Kosa(E)
}
```

The type parameter is available in field and variant types.

## Sifa (Traits)

Declare a trait (interface) with `sifa`:

```asili
sifa Inayoonyeshwa {
  kazi onyesha(hii: Self) -> Neno
}
```

Implement a trait for a struct using `shughuli ya ... kwa ...`:

```asili
umbo Paka { jina: Neno }

shughuli ya Paka kwa Inayoonyeshwa {
  kazi onyesha(hii: Paka) -> Neno {
    rejesha "Paka(" + hii.jina + ")"
  }
}
```

> Note: Trait dispatch is currently structural (duck typing at runtime). Full static dispatch is in progress.

## Waendeshaji wa Muundo (Compound Assignment Operators)

```asili
weka n = 10
n += 5    # n = 15
n -= 3    # n = 12
n *= 2    # n = 24
n /= 4    # n = 6
```

## Mikopo (Borrows)

`azima` creates an immutable borrow; `azima_tenda` creates a mutable borrow:

```asili
weka x = 42
weka r = azima x          # immutable reference
weka rm = azima_tenda x   # mutable reference
```

> Note: Borrow semantics are parsed and represented in the AST. Runtime enforcement is in progress.

## Kuacha Vigeuzi (Drop)

`tupa varname` explicitly drops a variable from scope (when followed by an identifier, not parentheses):

```asili
weka faili = soma_faili("data.txt")
# ... use faili ...
tupa faili    # explicitly released
```

## Mifano ya Linganisha ya Hali ya Juu (Advanced Match Patterns)

### Muundo (Struct Destructuring)

```asili
umbo Pika { x: Namba, y: Namba }
weka p = Pika { x: 3, y: 4 }

linganisha p {
  Pika { x: 0, y: _ } => { chapisha("kwenye mstari wa y") }
  Pika { x: a, y: b } => { chapisha("(" + (a kama Neno) + ", " + (b kama Neno) + ")") }
}
```

### Jenum (Enum Destructuring)

```asili
jenum Jibu { Sawa(Namba); Kosa(Neno) }

weka j = Jibu::Sawa(42)

linganisha j {
  Jibu::Sawa(n) => { chapisha("Jibu: " + (n kama Neno)) }
  Jibu::Kosa(e) => { chapisha("Kosa: " + e) }
  _             => {}
}
```

### Jozi (Pair Destructuring)

```asili
weka p = jozi(10, "moja")

linganisha p {
  (0, _)    => { chapisha("sifuri kwanza") }
  (n, neno) => { chapisha((n kama Neno) + " ni " + (neno kama Neno)) }
}
```

### Mifano ya Ukweli

```asili
weka b = kweli
linganisha b {
  kweli    => { chapisha("ndio") }
  si_kweli => { chapisha("hapana") }
}
```

## Sifa za Kazi (Function Attributes)

```asili
#[jaribio]
kazi mtihani_jumla() -> Ukweli {
  rejesha 2 + 2 == 4
}

#[sharti(os = "linux")]
kazi tekeleza_linux() -> Tupu {
  # only compiled on linux
}

#[ndani]
kazi msaidizi_wa_ndani() -> Tupu {
  # internal, not exported
}

#[kiunganishi]
kazi kutoka_c() -> Namba {
  # FFI linkage
}
```

| Sifa          | Maelezo                                         |
|---------------|-------------------------------------------------|
| `#[jaribio]`  | Mark as a test function (run by `pata jaribu`)  |
| `#[sharti(...)]` | Conditional compilation (e.g. `os = "linux"`) |
| `#[ndani]`    | Internal / not exported                          |
| `#[kiunganishi]` | FFI linkage annotation                        |
