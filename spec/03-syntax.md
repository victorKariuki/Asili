# 3. Syntax (Signal)

Previous: [Architecture and Files](02-architecture-and-files.md) | [Overview](../SPECIFICATION.md) | Next: [Type System](04-type-system.md)

---

## Syntactic specification (Signal)

| Category | Swahili | Technical mapping |
|----------|---------|--------------------|
| Variable | `weka` / `thabiti` | Mutable / Immutable |
| Logic | `kazi` / `rejesha` | Function / Return |
| Structure | `umbo` / `shughuli ya` | Struct / Impl (traits) |
| Flow | `ikiwa`, `au_ikiwa`, `vinginevyo` | If, Else If, Else |
| Loops | `wakati`, `kwa` | While, For |
| Types | Namba, Neno, Ukweli, Orodha, Kamusi | f64, String, Bool, Vec, HashMap |
| Empty | `Tupu`, `Hamna` | Void (type), Null (value) |
| Systems | `wazi`, `biti`, `sogeza_kushoto`, `sogeza_kulia` | Unsafe, Bitwise, Shift left/right |
| Bitwise | `na_biti`, `au_biti`, `xor_biti`, `siyo_biti` | AND, OR, XOR, NOT |
| Parallel | `tenda` | Green threads / spawn |
| Control flow | `linganisha`, `vunja`, `endelea`, `lebo` | Match, Break, Continue, Label |
| Error propagation | `jaribu` | Try / propagate (`?` operator) |
| Panic | `paparika` | Unrecoverable abort (panic) |

---

## Operator catalog

To keep Signal and Substrate aligned, operators are grouped by semantics.

### Arithmetic operations

Primary numeric operators for `Namba` and fixed-width `Biti*`/`uBiti*` types:

| Operator | Name | Notes |
|----------|------|-------|
| `+` | Jumla | Addition |
| `-` | Tofauti | Subtraction |
| `*` | Zao | Multiplication |
| `/` | Gawio | Division. For `Namba`, follows IEEE 754 (`Ukomo`, `-Ukomo`, `Siyo_Namba` where applicable). |
| `%` | Baki | Remainder |
| `**` | Kipeo | Exponentiation |

### Comparison operations

All comparisons evaluate to `Ukweli` (`kweli` or `si_kweli`):

`==`, `!=`, `>`, `<`, `>=`, `<=`.

### Logical operations

Logical composition over `Ukweli`:

- `na` (AND)
- `au` (OR)
- `siyo` (NOT)

### Bitwise operations

Bitwise operators for integer/fixed-width bit types:

- `na_biti` (AND)
- `au_biti` (OR)
- `xor_biti` (XOR)
- `siyo_biti` (NOT / inversion)
- `sogeza_kushoto` (left shift)
- `sogeza_kulia` (right shift)

### Assignment and compound assignment

- `=` assigns a value (move by default; implicit copy only for `Nakala` types).
- Compound forms are supported: `+=`, `-=`, `*=`, `/=`.

### Special protocol operations

- `kama` — explicit cast.
- `jaribu` / `?` — propagate `KOSA` in `Tokeo<T, E>`.
- `azima` / `azima_tenda` — immutable/mutable borrowing.
- `tupa` — explicit early drop.
- `paparika` — unrecoverable panic.

---

## Advanced control flow

- **`linganisha` (Match):** Structural pattern matching; destructure `umbo` or `Tokeo` and branch on variants.
- **`vunja` (Break):** Exit a loop early.
- **`endelea` (Continue):** Skip to the next iteration of a loop.
- **`lebo` (Label):** Label a loop for breaking out of nested loops (e.g. `vunja 'nje`).

---

## Error propagation

**`jaribu`** — Syntax sugar for early return of errors. Used with **Tokeo\<T, E\>**: unwraps `SAWA(thamani)` or returns the `KOSA` variant (e.g. `?` operator). Exact syntax (e.g. `jaribu { ... }` or `?` suffix) is fixed at implementation time. See [Resolved Decisions](08-resolved-decisions.md).

---

## Panic

**`paparika`** — Triggers an unrecoverable panic (halt condition). Intended only for critical failures (e.g. internal invariants, unsafe UB), not for normal error handling. Panics cannot be caught in Asili; see [Resolved Decisions](08-resolved-decisions.md).

---

## Entry point

Programs have a single entry:

```text
kazi kuu(hoja: Orodha<Neno>) -> Tupu
```

CLI arguments are always passed as `hoja`; the implementation may ignore them if not needed.

---

## Imports

- **Full module:** `leta mfumo`
- **Selective:** `leta mfumo::{chapisha, toka}`

---

## Loops

- **Iterator:** `kwa x katika orodha` — iterate over a collection.
- **Range:** `kwa i kutoka 0 hadi n` — numeric range sugar.
- **Unbounded loop:** `wakati milele { ... }` — intentional infinite loop form; must be exited via `vunja`, `rejesha`, or `paparika`.

---

## Bitwise and shift

- **Shift:** `sogeza_kushoto`, `sogeza_kulia`.
- **Bitwise logic:** `na_biti`, `au_biti`, `xor_biti`, `siyo_biti` (keyword family `biti`).

---

## Precedence note

Operator precedence follows conventional PEMDAS/BODMAS ordering. The cast operator `kama` binds tighter than arithmetic, so `a + b kama Biti32` parses as `a + (b kama Biti32)`.

---

Previous: [Architecture and Files](02-architecture-and-files.md) | [Overview](../SPECIFICATION.md) | Next: [Type System](04-type-system.md)
