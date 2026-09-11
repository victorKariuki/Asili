# 5. Standard Library (Msingi / Mfumo)

Previous: [Type System](04-type-system.md) | [Overview](../SPECIFICATION.md) | Next: [Tooling and Ecosystem](06-tooling-and-ecosystem.md)

---

## Principles

The Asili Standard Library is governed by the EDP. Its purpose is to provide a robust **Substrate** and minimize **Friction** between thought and execution.

- **Urahisi (Simplicity):** Core functions use intuitive Swahili terminology.
- **Nguvu (Performance):** Abstractions are zero-cost where possible; the library should not impose unnecessary cost on systems-level code.
- **Umoja (Consistency):** Naming conventions are uniform across all modules.
- **Explicit failure:** Functions that can fail return **Tokeo\<T, E\>**; no silent failure.
- **Portability:** The **Msingi (Core)** module is **no_std** compatible for bare-metal/embedded use.

---

## Moduli ya Msingi (Core — no_std)

Default-imported. No OS dependency. The Phase I interpreter provides selected stdlib as **builtin modules** (e.g. `hisabati`, `mfumo`) resolved without disk I/O; `leta hisabati` and `leta mfumo` work without corresponding `.asi` files. The same modules are also exposed as an **in-language surface** in **lib/std/** (`.asi`/`.as`), so the stdlib is both built-in and “written in the language.” Implementation may be split by module (hisabati, mfumo, neno, …) for scalability without changing the specified API.

### Hisabati (Math)

- **Infallible:** `jumla(a, b)`, `tofauti(a, b)`, `zao(a, b)` — basic arithmetic (`Namba` follows IEEE 754; fixed-width `Biti*`/`uBiti*` operators wrap deterministically).
- **Fallible:** `gawio(a, b)`, `kipeo(n, p)`, `mizizi(n)` — return **Tokeo\<Namba, Kosa\>** (e.g. divide by zero, negative sqrt).
- **Infallible helpers:** `duara(n)` (rounding), `absolute(n)` (absolute value) return `Namba`.
- **Special numeric constants:** `Ukomo` (positive infinity) and `Siyo_Namba` (NaN) are exposed for numeric logic on `Namba`.

### Neno (String)

- `neno.urefu()` — length in **grapheme clusters** (Lugha-Mama).
- `neno.biti_ngapi()` — length in **bytes** (Nguvu).
- `neno.unganisha()`, `neno.kata()`, `neno.tafuta()`.

### Orodha (List/Array)

- `orodha.ongeza()`, `orodha.ondoa()`, `orodha.kila_mmoja()`.

### Kasha_GC\<T\> (opt-in managed memory)

Not default-imported — requires `leta kasha_gc`. A minimal, deliberately small reference-counted
wrapper (`Rc<RefCell<Value>>`), the concrete realization of the "managed/GC modules" concept in
[07-execution-and-roadmap.md](07-execution-and-roadmap.md). Sharing is explicit via
`.shirikisha()` (like Rust's `Rc::clone`) — a plain `weka b = a` still moves, so wrapping in
`Kasha_GC<T>` never silently changes the language's default move semantics. Not a
tracing/cycle-collecting GC: a self-referential `Kasha_GC<T>` leaks, by design.

---

## Moduli ya Mfumo (System)

Requires `leta mfumo`. OS-dependent.

### Ingizo/Tokeo (I/O)

- `chapisha(ujumbe)` — print to stdout.
- `omba(swali)` — read from stdin with prompt.
- `soma_faili(njia)`, `andika_faili(njia, data)`.

### Mazingira (Environment)

- `mfumo.majira()`, `mfumo.vigezo()`, `mfumo.pata_env(jina)`, `mfumo.toka(kodi)`.

---

### Resource handles and traits

System resources are represented by explicit handle types that own their underlying OS or hardware resource and release it on drop:

- `Faili` — file handle for filesystem operations.
- `Mkondo` — network stream or socket.
- `Kumbukumbu<T>` — heap-allocated box owning a value of type `T`.

These types implement deterministic cleanup: when the owner goes out of scope (or `tupa` is called), the underlying handle is closed or freed.

Common behaviours are expressed via Sifa (traits), for example:

- `Inasomeka` — any type that can be read from (e.g. `Faili`, `Mkondo`).
- `Inandikika` — any type that can be written to.

These traits allow generic I/O functions to work over multiple resource types while preserving ownership and borrowing rules.

---

## Moduli ya Takwimu na Safu (Data)

High-performance primitives for multidimensional data, statistics, and large-scale processing.

### Tensors and linear algebra

- `safu.umbo(dim)`, `safu.zao_dot(a, b)`, `safu.geuza()`, `safu.kawaida()`.

### Transformation and calculus

- `safu.gradienti(kazi)`, `safu.takwimu(aina)`, `safu.badili(asili, lengo)`.

### Data loading

- `pakia.csv(njia)`, `pakia.jozi(njia)`, `pakia.gawanya(data, uwiano)`.

---

## Hardware / Nguvu (inside `wazi`)

Available inside **wazi** blocks for hardware-level control.

- `vuta_vifaani(data)` — offload to accelerators (GPU/NPU/DSP).
- `anwani_ya(kitu)` — memory address (pointer).
- `tenga_kumbukumbu(saizi)` — manual heap allocation.
- `piga_pini(namba, hali)` — GPIO for sensor data.

Typed hardware handles may be provided to model hardware state safely:

- `Pini` — GPIO pin configured as input or output.
- `Bafa<T>` — contiguous buffer with alignment guarantees suitable for accelerators (GPU/NPU).

---

## Error model

- **Success:** `SAWA(thamani)`.
- **Failure:** `KOSA(maelezo)` — Mwalimu-friendly message; type is **Tokeo\<T, E\>** with user-definable `E`.

Panic (`paparika`) is reserved for unrecoverable critical conditions (for example, internal invariants or undefined behaviour detected inside `wazi` blocks). Recoverable errors in the standard library must be expressed via `Tokeo\<T, E\>` and resolved using `linganisha` or `jaribu`, not via panic.

---

## Concurrency primitives (tenda, njia, fungo)

Used with **tenda** (green threads) for multi-threaded execution.

- **njia (Channel):** Send data between threads (e.g. `njia.unda()`, `njia.tuma()`, `njia.pokea()`). Exact API is defined at implementation.
- **fungo (Mutex/Lock):** Protect shared data from data races (e.g. `fungo.funga()`, `fungo.fungua()`).

Full concurrency model (M:N vs 1:1, happens-before) is fixed in [Resolved Decisions](08-resolved-decisions.md) when the feature is implemented.

---

## Async (sawia, subiri)

**sawia** (async) and **subiri** (await) — Future surface for non-blocking execution. Full semantics are deferred to [Execution and Roadmap](07-execution-and-roadmap.md) and 08 when the async phase is defined; they require an executor/runtime.

---

Previous: [Type System](04-type-system.md) | [Overview](../SPECIFICATION.md) | Next: [Tooling and Ecosystem](06-tooling-and-ecosystem.md)
