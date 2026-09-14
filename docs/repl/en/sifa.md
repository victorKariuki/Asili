# Sifa (traits) — REPL help

`sifa` (see also `?umbo`) is a **top-level item** — like `kazi`, `umbo`, `jenum`, and `leta`, it
cannot be typed directly at the REPL prompt. Typing `sifa Inayoonyeshwa { kazi onyesha(self:
Self) -> Neno }` at `>` fails to parse.

To work with traits, define them in a `.as` file and run it with `pata jenga --tenda`:

```asili
leta matumizi

sifa Inayoonyeshwa {
  kazi onyesha(self: Self) -> Neno
}

umbo Paka { jina: Neno }

shughuli ya Paka kwa Inayoonyeshwa {
  kazi onyesha(self: Paka) -> Neno {
    rejesha "Paka(" + self.jina + ")"
  }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka pk = Paka { jina: "Whiskers" }
  chapisha(pk.onyesha())          # Paka(Whiskers)
}
```

Two equivalent syntaxes for `shughuli ya`:

```asili
shughuli ya Paka kwa Inayoonyeshwa { ... }   # "kwa" keyword
shughuli ya Paka: Inayoonyeshwa { ... }      # colon
```

## Method completeness

A trait declares required methods (name, params, return type — no body). Every `shughuli ya`
that names a trait is checked against that list; a missing or mismatched method is a
parse/compile-time error, not a runtime one:

```asili
sifa Inayoonyeshwa {
  kazi onyesha(self: Self) -> Neno
  kazi jina_fupi(self: Self) -> Neno
}

umbo Paka { jina: Neno }

shughuli ya Paka kwa Inayoonyeshwa {
  kazi onyesha(self: Paka) -> Neno { rejesha self.jina }
  # jina_fupi is missing
}
```

This produces `[semantiki:SEM105] njia 'jina_fupi' ya sifa 'Inayoonyeshwa' haijatekelezwa kwa
'Paka'` before anything runs.

**Note:** completeness is only checked against real, user-written `shughuli ya` blocks. There is
no way to say "any value implementing trait X" generically (no `dyn`/trait objects) today — a
trait only decides which method a `.method()` call resolves to, not typed function parameters.

## See also

- `?umbo` — why top-level items don't work in the REPL
- `?faili` — `Faili`/`Mkondo` implement `Inasomeka`/`Inandikika` "by fiat," not through a real
  `shughuli ya` block (they aren't `umbo`)
