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

umma jenum Rangi { Nyekundu, Kijani, Bluu }
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
  Sawa(T),
  Kosa(E),
}
```

The type parameter is available in field and variant types.

## Sifa (Traits)

Declare a trait (interface) with `sifa`:

```asili
sifa Inayoonyeshwa {
  kazi onyesha(self: Self) -> Neno
}
```

Implement a trait for a struct using `shughuli ya ... kwa ...`:

```asili
umbo Paka { jina: Neno }

shughuli ya Paka kwa Inayoonyeshwa {
  kazi onyesha(self: Paka) -> Neno {
    rejesha "Paka(" + self.jina + ")"
  }
}
```

Calling a method defined inside `shughuli ya X kwa Trait` (e.g. `pk.onyesha()` above) works —
this used to fail with `SEM040: njia 'onyesha' haipo kwa 'Paka'` because `parse_impl_decl`
(`core/parser/src/parse.rs`) had `target`/`trait_name` swapped for the `kwa` syntax specifically
(the colon syntax, `shughuli ya Target: Trait { }`, was always correct). Fixed.

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
```

> Note: conflicting borrows are already caught at **compile time** (semantic analysis), not
> just parsed/represented — e.g. taking both `azima x` and `azima_tenda x` for the same `x` in
> the same scope raises `SEM043: migongano ya borrowing kwa: x`. The move/borrow checker
> (`Binding` tracking in `core/parser/src/semantic/analyzer.rs`) is real and enforced today;
> what's still missing is a runtime backing for moves (the evaluator clones regardless) — see
> [implementation-status.md](../design/implementation-status.md).

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
  _ => {}   # exhaustiveness checker requires this even though the two arms above cover p
}
```

### Jenum (Enum Destructuring)

```asili
jenum Jibu { Sawa(Namba), Kosa(Neno) }

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
  _ => {}   # exhaustiveness checker requires this even though the two arms above cover p
}
```

### Mifano ya Ukweli

```asili
weka b = kweli
linganisha b {
  kweli    => { chapisha("ndio") }
  si_kweli => { chapisha("hapana") }
  _ => {}   # the exhaustiveness checker doesn't recognize kweli/si_kweli as covering all
            # of Ukweli, so this is required even though the two arms above are exhaustive
}
```

## Sifa za Kazi (Function Attributes)

```asili
#[jaribio]
kazi mtihani_jumla() -> Ukweli {
  rejesha 2 + 2 == 4
}

#[sharti(lengo = "native")]
kazi tekeleza_native() -> Tupu {
  # only compiled when building for the "native" target
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
| `#[kabla]`    | Setup fixture: runs before every `#[jaribio]` test in the same file (`pata jaribu`) |
| `#[baada]`    | Teardown fixture: runs after every `#[jaribio]` test in the same file, even if the test failed (`pata jaribu`) |
| `#[sharti(...)]` | Conditional compilation; only key `lengo` is recognized (e.g. `lengo = "wasm"`), values OR'd with `\|` — see [implementation-status.md](../design/implementation-status.md) |
| `#[ndani]`    | Internal / not exported                          |
| `#[kiunganishi]` | FFI linkage annotation                        |
