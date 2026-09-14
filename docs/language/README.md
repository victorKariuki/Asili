# Lugha ya Asili — Marejeo (Language Reference)

| Faili               | Mada                                              |
|---------------------|---------------------------------------------------|
| [01-misingi.md](01-misingi.md) | Variables, types, casting, operators   |
| [02-kazi.md](02-kazi.md)       | Functions, entry point, methods        |
| [03-udhibiti.md](03-udhibiti.md) | if/else, match, loops, break/continue |
| [04-muundo-data.md](04-muundo-data.md) | Orodha, Kamusi, Jozi, Structs, Enums |
| [05-makosa.md](05-makosa.md)   | Chaguo, Tokeo, jaribu, ?, tupa        |
| [06-moduli.md](06-moduli.md)   | leta, standard library modules         |
| [07-mfumo-wa-aina.md](07-mfumo-wa-aina.md) | Generics, traits, visibility, advanced patterns, attributes |
| [08-njia-za-aina.md](08-njia-za-aina.md) | All built-in methods: Neno, Orodha, Kamusi, Jozi, Chaguo, Tokeo |
| [09-waendeshaji.md](09-waendeshaji.md) | Operator precedence table, unary ops, special values, bitwise |
| [10-mifano.md](10-mifano.md)   | Annotated examples from the examples/ directory             |
| [11-maadili-ya-kimataifa.md](11-maadili-ya-kimataifa.md) | Global constants always in scope (no `leta` needed) |

## Mwanzo wa Haraka (Quick Start)

```bash
# Install
cargo build --release
cp target/release/pata ~/.local/bin/

# New project
pata njozi mradi_wangu
cd mradi_wangu

# Edit src/kuu.as, then run
pata jenga --tenda

# Interactive exploration
pata repl
```

## Hello World

```asili
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("Habari Dunia!")
}
```

## Mfano Mkubwa Zaidi

```asili
leta matumizi
leta hisabati

umbo Mwanafunzi {
  jina: Neno
  alama: Namba
}

kazi daraja(alama: Namba) -> Neno {
  ikiwa alama >= 90 { rejesha "A" }
  au_ikiwa alama >= 75 { rejesha "B" }
  au_ikiwa alama >= 60 { rejesha "C" }
  vinginevyo { rejesha "D" }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka wanafunzi = orodha(
    Mwanafunzi { jina: "Amara", alama: 88 },
    Mwanafunzi { jina: "Baraka", alama: 95 },
    Mwanafunzi { jina: "Zawadi", alama: 72 }
  )

  kwa mw katika wanafunzi {
    weka d = daraja(mw.alama)
    chapisha(mw.jina + ": " + d + " (" + (mw.alama kama Neno) + ")")
  }
}
```

Output:
```
Amara: B (88)
Baraka: A (95)
Zawadi: C (72)
```
