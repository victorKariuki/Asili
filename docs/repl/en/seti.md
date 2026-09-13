# Seti (set) — REPL help

`Seti` is a collection of unique values (no duplicates). Always in scope — no `leta` needed.

## Creating

```
> weka s = seti(1, 2, 3, 2)
> s.urefu()
Namba(3.0)
```

`seti(v1, v2, ...)` silently drops duplicates. `seti_tupu()` starts empty.

## Methods

| Expression    | Description                                |
|---------------|-----------------------------------------------|
| `s.ongeza(v)` | Add a member (a no-op if already present)  |
| `s.ondoa(v)`  | Remove; returns `Ukweli` — true if it was present |
| `s.ina(v)`    | `Ukweli`: is `v` a member?                 |
| `s.urefu()`   | Member count                               |
| `s.orodha()`  | Convert to `Orodha<T>`                     |
| `s.clona()`   | Clone                                      |

## Example

```
> weka s = seti(1, 2, 3, 2)
> s.ina(2)
Ukweli(true)
> s.ongeza(9)
> s.orodha()
Orodha([Namba(2.0), Namba(3.0), Namba(1.0), Namba(9.0)])
```

**Note:** `.orodha()`'s ordering is unspecified — `Seti` is backed by Rust's `HashSet`
internally, so don't rely on the same order across versions or even across runs.

## See also

- `?kamusi` — another collection built on the same `MapKey` type (key-value instead of set)
- `?orodha` — the type `.orodha()` converts into
