# Data structures example — coverage of gaps and limits

This example pushes Orodha, Kamusi, Jozi, Struct, Chaguo, Tokeo, and Neno to their limits. Below is how it maps to the “overlooked or under-specified” list.

---

## 1. Indexing and in-place updates

| Point | Covered? | How |
|-------|----------|-----|
| Indexing only for **Orodha** (`a[i]` → Tokeo(Ok/Err)) | Yes | `orodha_index_first`, `orodha_index_last`, `orodha_single`, `orodha_negative_index`, `tokeo_propagate_in_bounds` (a[1]?), `orodha_ondoa_oob` (Tokeo(Err) for index OOB not runnable in kuu). |
| No **Neno** indexing; use `kata` / `tafuta` | Yes | Neno demos use `kata`, `tafuta`, `urefu`, `biti_ngapi` only — no `s[i]`. |
| No index/field **lvalues** (`a[i]=x`, `s.field=x`) | Yes | No such syntax. Workarounds shown: **struct** via `struct_rebuild` (new struct value); **orodha** via `a.ongeza(x)` / `a.ondoa(i)` (mutate variable). |

---

## 2. Orodha

| Point | Covered? | How |
|-------|----------|-----|
| **kila_mmoja** is a stub (returns Tupu, no callback) | Yes | `orodha_kila_mmoja()` calls it and prints `"Tupu"`. |
| No slice/subrange (`a.kata`, `a[1..3]`) | N/A | Not in language; only single-element access and `ondoa`. |
| No “insert at index” or “contains” for lists | N/A | Not in language. |
| **Negative index** clamped to 0 | Yes | `orodha_negative_index()`: `a[0-1]?` → first element. |
| **Empty list** `orodha()` valid | Yes | `orodha_empty()`: `orodha()`, then `urefu()` → 0. |

---

## 3. Kamusi

| Point | Covered? | How |
|-------|----------|-----|
| Only **ingiza** / **pata** (no funguo, maana, ondoa) | Yes | All demos use only `kamusi_tupu`, `ingiza`, `pata`. |
| **Key types**: Neno, Namba, Ukweli, Herufi only | Yes | `kamusi_neno_key`, `kamusi_namba_key`, `kamusi_ukweli_key`, `kamusi_herufi_key`. No Jozi/Struct/Orodha as key (not supported). |

---

## 4. Jozi

| Point | Covered? | How |
|-------|----------|-----|
| Only **kwanza** / **pili**; no destructuring in checker | Yes | All access via `p.kwanza()`, `p.pili()` (and casts). Pattern `(a,b)` in `linganisha` does not bind in semantic (SEM045). |
| No n-tuple; **jozi of jozi** nesting | Yes | `jozi_nested()`: `jozi(jozi(10,20), 30)` and inner `kwanza`/`pili`. |

---

## 5. Struct

| Point | Covered? | How |
|-------|----------|-----|
| **Read-only** field access; no field assignment | Yes | `struct_literal`, `struct_field_neno` read; “update” only via **struct_rebuild** (new value). |
| **Copy/move**: structs are values; passing moves | Yes | `struct_pass_move(p: Pika)`: pass struct by value, use `p.x`. |

---

## 6. Chaguo / Tokeo

| Point | Covered? | How |
|-------|----------|-----|
| No built-in unwrap/default; **chaguo_au_namba** in mfumo | Yes | `chaguo_au_namba_some()` uses `chaguo_au_namba(m.pata("x"), 0)`. |
| Tokeo consumed by `?` and **jaribu**; no map/and_then | Yes | `tokeo_ok()` (jaribu), `tokeo_propagate_in_bounds()` (a[1]?). No map/and_then (not in language). |

---

## 7. Types in type system only (no runtime)

| Point | Covered? | How |
|-------|----------|-----|
| **Mfululizo&lt;T&gt;** and **Seti&lt;T&gt;** in parser/semantic only | N/A | No `Value::*` or builtins; cannot demonstrate in a runnable example. |

---

## 8. String as data structure

| Point | Covered? | How |
|-------|----------|-----|
| **urefu**, **biti_ngapi**, **unganisha**, **kata**, **tafuta** | Yes | All used in neno_* functions. |
| **kata** byte ranges vs **urefu** grapheme | Yes | `neno_grapheme_urefu()` (é → 1 grapheme), `neno_byte_kata()` (é bytes). |

---

## 9. Mutation and receiver

| Point | Covered? | How |
|-------|----------|-----|
| **ongeza** / **ondoa** / **ingiza** only mutate when receiver is an **identifier** | Partial | Implemented in evaluator (only `Expr::Ident(receiver)` gets `env.set`). Not runnable: `(a).ongeza(3)` is rejected (SEM038); `weka b = a; b.ongeza(3); a.urefu()` is invalid (SEM040 use-after-move). Mutation is shown by `orodha_urefu_after_ongeza` etc. |

---

## 10. Spec vs implementation

| Point | Covered? | How |
|-------|----------|-----|
| 05-standard-library.md omissions (a[i], urefu(), empty orodha(), Kamusi/Jozi) | N/A | Documentation only; not something this example can “cover”. |

---

**Run:** `cd examples/data_structures && cargo run -p pata-cli -- jenga --tenda`
