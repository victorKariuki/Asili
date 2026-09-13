# Sambamba (concurrency) — REPL help

`tenda`/`njia`/`fungo` are top-level items (or need top-level `kazi` to call) — `kazi` can't be
typed at the REPL prompt, so these examples must be written in a `.as` file and run with
`pata jenga --tenda` (see `?kazi`).

1:1 real-OS-thread model — one `tenda` spawns one OS thread, not a green thread.

## tenda / subiri_tenda

```asili
leta sambamba
leta matumizi

kazi mfanyakazi(tx: NjiaTx<Namba>, n: Namba) -> Tupu {
    jaribu (tx.tuma(n * n))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka p = njia()
    weka tx = p.kwanza()
    weka rx = p.pili()

    weka id = jaribu (tenda("mfanyakazi", tx, 5))
    weka jibu = jaribu (rx.pokea())
    chapisha(jibu kama Neno)          // "25"
    jaribu (subiri_tenda(id))
}
```

**Important:** a spawned function's own return value does **not** come back through
`subiri_tenda` directly — `subiri_tenda(id)` only reports `Tokeo<Tupu, Neno>` (did the thread
finish or panic). Use `njia` to send the real result back.

`Kasha_GC<T>`/`Faili`/`Mkondo` can't be passed as `tenda` arguments — rejected with `Kosa`.

## Njia

| Method        | Description                                          |
|---------------|---------------------------------------------------------|
| `njia()`      | Create a new `NjiaTx<T>`/`NjiaRx<T>` pair as a `Jozi` — can't fail |
| `tx.tuma(v)`  | Send; `Tokeo<Tupu, Neno>`                            |
| `rx.pokea()`  | Receive (blocks); `Tokeo<T, Neno>` — `Kosa` once every sender closes |

## Fungo

```asili
leta sambamba

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka f = jaribu (fungo(0.0))
    f.weka(42.0)
    chapisha(f.pata() kama Neno)   // "42"
}
```

| Method        | Description                                          |
|---------------|---------------------------------------------------------|
| `fungo(v)`    | Wrap `v`; `Tokeo<Fungo<T>, Neno>`                    |
| `f.pata()`    | Lock+read+unlock — the safe one-step way             |
| `f.weka(v)`   | Lock+write+unlock — the safe one-step way            |
| `f.funga()` / `f.fungua()` | Explicit lock/unlock for holding across several operations — `.fungua()` without a matching prior `.funga()` is a panic |

## See also

- `?kazi` — why top-level items don't work in the REPL
- `?faili` — the other handle types `tenda`/`fungo` reject
