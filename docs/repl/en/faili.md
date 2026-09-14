# Faili (file handle) — REPL help

`Faili` is a file handle — it closes automatically when it goes out of scope, even without an
explicit `.funga()` or `tupa`. `Mkondo` (TCP stream) and `Kumbukumbu<T>` (heap box) are related
handle types with a similar method shape.

**Note:** `faili_fungua`/`mkondo_unganisha` require `leta faili`/`leta mfumo`, and `leta` cannot
be typed directly at the REPL prompt (see [?kazi](kazi.md) for why). So these examples must be
written in a `.as` file and run with `pata jenga --tenda`, not typed directly at `>`.
`kumbukumbu_unda` alone needs no `leta` (it's in the prelude) and can be tried directly at the
REPL — see the Kumbukumbu section below.

## Faili

```asili
leta faili

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka w = jaribu (faili_fungua("data.txt", "andika"))
  jaribu (w.andika("habari"))
  w.funga()

  weka r = jaribu (faili_fungua("data.txt", "soma"))
  chapisha(jaribu (r.soma()))   # "habari"
}
```

| Method          | Description                                   |
|-----------------|------------------------------------------------|
| `faili_fungua(njia, hali)` | Open; `hali` is `"soma"`, `"andika"`, or `"ongeza"`; returns `Tokeo<Faili, Neno>` |
| `w.soma()`      | Read all as `Neno`; returns `Tokeo<Neno, Neno>` |
| `w.andika(data)`| Write `data`; returns `Tokeo<Tupu, Neno>`     |
| `w.funga()`     | Explicit close (a no-op if already closed)    |

## Mkondo

```asili
leta mfumo

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka m = jaribu (mkondo_unganisha("127.0.0.1:8080"))
  jaribu (m.andika("GET / HTTP/1.0\r\n\r\n"))
  chapisha(jaribu (m.soma()))
  m.funga()
}
```

Same methods as `Faili`: `.soma()`, `.andika(data)`, `.funga()`. Also:

| Method | Description |
|--------|-------------|
| `m.soma_bailisi(kikomo)` | Read **once, not to EOF** — up to `kikomo` bytes; returns `Tokeo<Neno, Neno>`. Needed for protocols like HTTP/1.1 keep-alive that must read one message and then read again on the same connection, rather than `.soma()`'s wait-for-EOF |

**Listening (server) and TLS**: `mkondo_sikiliza`, `mkondo_tumikia`, `mkondo_tumikia_http`,
`tls_sanidi` — see `docs/language/06-moduli.md` (the `mfumo` section) and
`docs/design/http-server-design.md`.

**JSON**: `kwa_json`/`kutoka_json` (`Value` ↔ JSON `Neno`) — see `docs/language/06-moduli.md` and
`docs/design/json-codec-design.md`.

## Kumbukumbu\<T\>

This one **can** be tried directly at the REPL — no `leta` needed:

```
> weka k = kumbukumbu_unda(42.0)
> k.pata()
Namba(42.0)
```

| Method              | Description                      |
|---------------------|-----------------------------------|
| `kumbukumbu_unda(v)` | Box `v` into a new `Kumbukumbu<T>` |
| `k.pata()`          | Read a clone of the boxed value    |

**No `.weka()`** — `Kumbukumbu<T>` doesn't share like `Kasha_GC<T>` does (see
`docs/language/06-moduli.md`); mutating requires reassigning the whole binding:
`weka k = kumbukumbu_unda(newval)`.

## See also

- `?kazi` — why `leta` can't be typed at the REPL prompt
- `?makosa` — Tokeo and how to consume it (`jaribu`, `linganisha`)
