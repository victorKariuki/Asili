# 8. Resolved Decisions (Conflicts and Open Questions)

Previous: [Execution and Roadmap](07-execution-and-roadmap.md) | [Overview](../SPECIFICATION.md)

---

These are the definitive resolutions for the Asili v1.1 **Substrate**, following **Explicit Failure** and **Zero-Cost Abstractions**.

---

## 9.1 Types and semantics

| Topic | Decision |
|-------|----------|
| **Tokeo** | **Tokeo\<T, E\>**. User-defined error types are supported; `KOSA(maelezo)` is the common case for simple errors. |
| **Namba → Neno** | Via trait **Onyesheka** (Display). Standard types (e.g. `Namba`) implement it; `x kama Neno` uses this. |
| **String length** | **Neno.urefu()** returns **grapheme clusters** (Lugha-Mama). **Neno.biti_ngapi()** returns **bytes** (Nguvu). |
| **Math fallibility** | **Infallible:** `jumla`, `tofauti`, `zao`, `duara`, `absolute`. **Fallible:** `gawio`, `kipeo`, `mizizi` return **Tokeo\<Namba, Kosa\>**. |
| **Literal default** | Integer literal `2026` has type **Namba (f64)** unless context requires a different numeric type. |
| **Floating-point edges** | `Namba` follows IEEE 754; `Ukomo`, `-Ukomo`, and `Siyo_Namba` are first-class numeric states for overflow/undefined floating-point results. |
| **Friction vocabulary** | `Mfuriko` names overflow and `Ufinyu` names underflow in diagnostics/error payloads for numeric operations on constrained/fixed-width types. |
| **Fixed-width arithmetic semantics** | Arithmetic on `Biti*`/`uBiti*` uses deterministic two's-complement wrapping in core operators. APIs that opt into checked arithmetic must surface overflow/underflow explicitly via `Tokeo` (e.g. `KOSA(Mfuriko)` / `KOSA(Ufinyu)`). |
| **Recoverable errors** | Recoverable failures must be expressed as data using **Tokeo\<T, E\>** (and optionally **Chaguo\<T\>**/`T?`), not as exceptions. |
| **Panic (`paparika`)** | `paparika` is an unrecoverable panic (halt). It cannot be caught in Asili and is reserved for critical conditions only; regular error handling must use `Tokeo`. |

---

## 9.2 Syntax and keywords

| Topic | Decision |
|-------|----------|
| **Bitwise / shift** | **Shift:** `sogeza_kushoto`, `sogeza_kulia`. **Bitwise:** `na_biti`, `au_biti`, `xor_biti`. |
| **Operator families** | Arithmetic (`+`, `-`, `*`, `/`, `%`, `**`), comparison (`==`, `!=`, `>`, `<`, `>=`, `<=`), logical (`na`, `au`, `siyo`), assignment (`=`, `+=`, `-=`, `*=`, `/=`) are part of core Signal. |
| **Bitwise NOT** | `siyo_biti` is the unary bitwise inversion operator for fixed-width integer/bit types. |
| **kwa loop** | **Iterator:** `kwa x katika orodha`. **Range:** `kwa i kutoka 0 hadi n`. Both supported. |
| **Unbounded loop form** | `wakati milele { ... }` is the canonical syntax for intentional infinite loops. |
| **Entry point** | **kazi kuu(hoja: Orodha\<Neno\>) -> Tupu**. CLI args always passed; may be ignored. |
| **Import syntax** | **Selective:** `leta mfumo::{chapisha, toka}`. **Full module:** `leta mfumo`. |

---

## 9.3 Naming and structure

| Topic | Decision |
|-------|----------|
| **Dereva vs Mfumo** | **Split.** **mfumo** = high-level System API (StdLib). **dereva** = internal HAL/FFI layer used to implement mfumo. |
| **Akili location** | For v1.1, a **submodule of Msingi**, under **Takwimu/Safu**. **Conditionally compiled** (excluded for minimal embedded targets). |

---

## 9.4 Process and tooling

| Topic | Decision |
|-------|----------|
| **Doc rejection** | **CI check.** `pata thibitisha` fails if any public `kazi` or `umbo` lacks a documentation string; patch rejected. |
| **Mainline branch** | **asili-mainline**. |

---

## 9.5 Casting and safety

| Topic | Decision |
|-------|----------|
| **kama on failure** | Returns **T?**. On failure (e.g. `1000 kama Biti8`), result is **Hamna**. Caller must handle nullability. |
| **Precedence** | **kama** binds **tighter** than arithmetic. So `a + b kama Namba` means `a + (b kama Namba)`. |

---

## 9.6 Memory strategy (Orodha allocation)

| Topic | Decision |
|-------|----------|
| **Standard mode** | When the OS or allocator denies a request (e.g. `Orodha::ongeza` cannot allocate), the runtime **Aborts**. No `Tokeo` is returned; failure is fatal. |
| **Embedded / strict mode** | In constrained or explicitly configured environments, **Orodha::ongeza** (and equivalent growth operations) may return **Tokeo\<Tupu, Kosa\>** on allocation failure. Memory exhaustion is then an explicit, handleable **Signal**. The mode is determined by target (e.g. embedded) or runtime/allocator configuration. |

---

## 9.7 Chaguo vs T?

| Topic | Decision |
|-------|----------|
| **Chaguo\<T\>** | Formal type with variants **Kuna(T)** and **Hamna**. |
| **T?** | Syntactic sugar for **Chaguo\<T\>**. **Hamna** is the value for the empty variant. One type, two spellings. |

---

## 9.8 Error propagation (jaribu, ?)

| Topic | Decision |
|-------|----------|
| **jaribu** | Block or construct for error propagation. |
| **?** operator | Unwraps **SAWA(thamani)** or returns the **KOSA** variant to the caller. Used with **Tokeo\<T, E\>**. Exact syntax (e.g. `jaribu { ... }` or `?` suffix) is fixed when implementing. |

---

## 9.9 Attributes

| Topic | Decision |
|-------|----------|
| **#[jaribio]** | Discovered by **pata jaribu** and run as unit tests. |
| **#[sharti(...)]** | Conditional compilation; code included only when the condition holds. |
| **#[kiunganishi]** | Marks FFI declarations (link to external C/C++ libraries). |
| **#[ndani]** | Optimization hint (e.g. inline). |
| **Resolution order** | After AST, before codegen. |

---

## 9.10 Ownership and borrowing

| Topic | Decision |
|-------|----------|
| **Ownership** | Every value has a single owner. Assignment and argument passing move ownership by default unless the type implements `Nakala` (Copy). After a move, the previous binding may no longer be used. |
| **Borrowing syntax** | `azima` and `azima_tenda` are the canonical keywords for immutable and mutable borrows. Symbols `&` and `&mut` are allowed shorthand. |
| **Reference types** | `Rejeo<T>` and `Rejeo_Tenda<T>` are the underlying immutable and mutable reference types; `azima`/`azima_tenda` desugar to these. |
| **Aliasing rules** | Many immutable borrows or one mutable borrow at a time; not both simultaneously. |
| **Drop (`tupa`)** | Owned values are dropped deterministically at end of scope. `tupa x` explicitly drops `x` early; `x` cannot be used afterwards. Resources such as `Faili` and `Mkondo` must release underlying OS/hardware handles on drop. |
| **Copy/clone** | Types implementing `Nakala` may be copied implicitly in limited contexts; other types must be cloned explicitly via `.nakala()`. |

---

## Deferred (algorithm/runtime details)

The ownership and borrowing language surface is fixed in 9.10. Deferred items are implementation details: the internal borrow-checker algorithm and full async runtime semantics for **sawia**/**subiri**.

---

## Implications

- **Type of `expr kama T` when fallible:** The expression has type **T?**, not T. Callers must handle `Hamna`.
- **Tokeo vs T?:** Use **Tokeo\<T, E\>** for operations that can fail with a *reason* (I/O, parsing). Use **T?** for "value or absent" (e.g. failed cast, missing env var).
- **Syntactic spec:** The Signal table includes `sogeza_kushoto`, `sogeza_kulia`, and `na_biti`, `au_biti`, `xor_biti`, `siyo_biti` as specified above.

---

Previous: [Execution and Roadmap](07-execution-and-roadmap.md) | [Overview](../SPECIFICATION.md)
