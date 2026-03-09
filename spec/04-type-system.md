# 4. Type System (Mfumo wa Aina)

Previous: [Syntax](03-syntax.md) | [Overview](../SPECIFICATION.md) | Next: [Standard Library](05-standard-library.md)

---

Types are the **Substrate** that ensures **Validation** before **Projection**.

---

## Numeric types (Aina za Namba)

### Safu ya Urahisi (ease / default)

| Swahili | Technical | Description |
|---------|------------|-------------|
| **Namba** | f64 | Default number. Handles both integers and decimals automatically. |

### Safu ya Nguvu (systems / fixed-precision)

Used inside `wazi` blocks or when performance/memory is critical. Map directly to hardware registers.

| Swahili | Technical |
|---------|------------|
| Biti8, uBiti8 | i8, u8 |
| Biti32, uBiti32 | i32, u32 |
| Biti64, uBiti64 | i64, u64 |

### Safu ya Ukubwa (scale / arbitrary precision)

| Swahili | Technical |
|---------|------------|
| **Namba_Kuu** | BigInt |
| **Namba_Sahihi** | BigDecimal |

---

## Other primitives

| Swahili | Technical | Notes |
|---------|------------|--------|
| **Ukweli** | bool | `kweli` / `si_kweli` |
| **Herufi** | char | Single Unicode scalar |
| **Tupu** | () | Unit/void type |

---

## Reference types

| Swahili | Technical |
|---------|------------|
| **Neno** | String (UTF-8, heap) |
| **Orodha\<T\>** | Vec\<T\> |
| **Kamusi\<K,V\>** | HashMap\<K,V\> |
| **Tokeo\<T, E\>** | Result\<T, E\> — user-defined error types supported; `KOSA(maelezo)` is the common case. |

---

## Nullability (Hali ya Hamna)

- **Explicit nullability:** `T?` denotes a type that may be absent.
- **Value:** `Hamna` is the "no data" value.
- Example: `weka x: Namba? = Hamna`

---

## Casting (Ubadilishaji)

- **Promotion:** e.g. `Biti32` → `Namba` is automatic where safe.
- **Demotion:** e.g. `Namba` → `Biti32` requires explicit cast and may fail; see Resolved Decisions.
- **Explicit cast:** keyword **`kama`** — e.g. `weka x = namba_kuu kama Biti64`.
- **Precedence:** `kama` binds **tighter** than arithmetic. So `a + b kama Namba` means `a + (b kama Namba)`.
- **Failure:** When a cast can fail (e.g. `1000 kama Biti8`), the result type is **`T?`** and the value is **`Hamna`** on failure. The caller must handle nullability explicitly.

---

## Literal default

Integer literal `2026` has type **Namba (f64)** unless context requires a different numeric type (e.g. `Biti32`, `Namba_Kuu`).

### Special floating-point values (`Namba`)

`Namba` follows IEEE 754 floating-point semantics:

- **`Ukomo`** — positive infinity (`+∞`).
- **`-Ukomo`** — negative infinity (`-∞`).
- **`Siyo_Namba`** — NaN / undefined floating-point result.

These values are valid `Namba` states and can arise from arithmetic such as division by zero or invalid operations (for example `0.0 / 0.0` producing `Siyo_Namba`).

### Overflow and underflow vocabulary

For diagnostics and error typing in fixed-width arithmetic:

- **`Mfuriko`** — overflow (value exceeds representable range).
- **`Ufinyu`** — underflow (value magnitude falls below representable non-zero precision).

For `Biti*`/`uBiti*` core operators, overflow/underflow uses deterministic two's-complement wrapping. Checked behaviour is available through APIs that return `Tokeo` with explicit friction terms (for example `Mfuriko` or `Ufinyu`), as defined in [Resolved Decisions](08-resolved-decisions.md).

---

## Onyesheka (Display trait)

Conversion from values to **Neno** (e.g. `Namba` → `Neno`) is defined via the trait **Onyesheka**. Standard types implement it by default; `x kama Neno` uses this trait.

---

## Abstraction (Sifa and generics)

- **Sifa (Trait/Interface):** Defines shared behaviour (e.g. `sifa Onyesheka`). Types implement Sifa via `shughuli ya`.
- **Jumla\<T\> (Generics):** Logic parameterised over type `T`; works for any type that satisfies the required bounds.

---

## Memory, ownership, and references

Asili uses **ownership and borrowing** as its default memory model. Every value has a single owner; assignment and argument passing **move** ownership by default unless a type implements the `Nakala` (Copy) trait.

- **Move semantics:** After a move, the previous binding may no longer be used.
- **Borrowing:** Instead of moving, code may create references:
  - `azima` — immutable borrow (desugars to `Rejeo<T>`).
  - `azima_tenda` — mutable borrow (desugars to `Rejeo_Tenda<T>`).
- **Aliasing rules:** Many immutable borrows or a single mutable borrow are allowed, but not both at the same time.
- **Drop (`tupa`):** When an owner goes out of scope, its value is dropped deterministically. The `tupa` operation explicitly drops a value early; it cannot be used afterwards.
- **Copy and clone:** Types that implement `Nakala` may be copied implicitly in some contexts; other types must be explicitly cloned via `.nakala()`.

Lifetimes are inferred by default; explicit lifetime annotations are only required when references cross complex structural boundaries (e.g. stored in `umbo` fields or returned from functions with non-obvious relationships). See [Execution and Roadmap](07-execution-and-roadmap.md) for the feature–phase map.

| Swahili | Technical | Notes |
|---------|------------|--------|
| **Rejeo** | `&T` | Immutable borrow (`azima`) |
| **Rejeo_Tenda** | `&mut T` | Mutable borrow (`azima_tenda`) |
| **Muda_wa_Kuishi** | Lifetimes | How long a reference remains valid |
| **Kiashiria** | Raw pointer | Used only in `wazi` blocks |
| **Gundi** | Static/global | Data that lives for the entire program |

---

## Data shapes (additional)

| Swahili | Technical | Notes |
|---------|------------|--------|
| **Chaguo\<T\>** | Option\<T\> | Variants `Kuna(T)` \| `Hamna`; see 08 for relation to `T?`. |
| **Mfululizo** | Slice `[T]` | View into `Orodha` or buffer |
| **Jozi** | Tuple | Grouping without a named `umbo`, e.g. `(Namba, Ukweli)` |
| **Seti** | Set | Unique collection of values |

---

## Modules (Pakiti / Moduli)

**Pakiti** / **Moduli** — Namespace and module management for large-scale code. Maps to file layout (see [Architecture and Files](02-architecture-and-files.md)) and to `leta` (see [Syntax](03-syntax.md)).

---

Previous: [Syntax](03-syntax.md) | [Overview](../SPECIFICATION.md) | Next: [Standard Library](05-standard-library.md)
