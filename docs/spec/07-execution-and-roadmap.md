# 7. Execution and Roadmap

Previous: [Tooling and Ecosystem](06-tooling-and-ecosystem.md) | [Overview](../SPECIFICATION.md) | Next: [Resolved Decisions](08-resolved-decisions.md)

---

## Execution pipeline

1. **Source** — `.as` and `.asi` (UTF-8).
2. **Lexer / Parser** — Written in Rust; produces AST.
3. **Type checker** — Strict-but-inferred (TypeScript-style).
4. **Bytecode** — Emit `.asb` for the VM.
5. **Execution** — VM for Terminal/Web; **Pata Jenga** produces native binaries via LLVM.

**Attributes (`#[...]`):** Resolved in the compiler pipeline after parse, before or during codegen. Conditional compilation (`#[sharti(...)]`) determines which code is included per target.

---

## Turing completeness

The **Signal** (syntax and type system) provides the components required for Turing completeness: conditional branching (`ikiwa`/`au_ikiwa`/`vinginevyo`), arbitrary iteration (`wakati`, `kwa`) and recursion (`kazi`/`rejesha`), and an arbitrarily large memory substrate (`Orodha`, `Kamusi`, `Namba`/`Biti`, bitwise operations). The language is therefore Turing complete. The **Execution** layer handles the resulting friction (non-termination, resource exhaustion) without polluting the Signal.

---

## Tail call semantics and recursion

Execution distinguishes tail calls from general recursion so that recursion in tail form does not grow the stack without bound.

- **Tail call requirement:** When the compiler (bytecode or native backend) can determine that a call is in **tail position** (the result of the call is the result of the function, with no further computation), it must implement that call as a **tail call**: reuse the current activation (frame) and transfer control to the callee so that recursion in tail position uses O(1) space. This applies to self-calls and mutual tail calls. The tree-walk interpreter does not perform TCO; compliance is required for the bytecode VM and native backends.
- **Non-tail recursion:** For calls not in tail position, execution remains bounded by the **depth/stack guard**. Overflow is reported as a managed error (e.g. "undani mno") rather than process abort.
- **Telemetry:** The runtime may expose **peak evaluation/call depth** during execution for development and validation; in production the same guard enforces the limit.
- **Iteration:** Explicit iteration (`wakati`, `kwa`) remains the primary way to express loops; tail-call semantics ensure that recursion used in tail form does not impose unbounded stack growth.

---

## Execution-level friction (EDP view)

These concerns are **Friction** at the **Execution** layer, not deficiencies in the Signal. Keeping them out of the core syntax preserves portability: the same program is logically valid on a server or a microcontroller; only the **Projection** (runtime result) may differ when resources are exceeded.

### Diagnostic view

| Component | EDP layer | Role |
|-----------|-----------|------|
| **Syntax / Types** | **Signal** | Defines what *can* be expressed (Turing completeness). |
| **Runtime / VM** | **Execution** | Manages the friction of infinite loops (time). |
| **Allocator** | **Substrate** | Manages the friction of finite resources (space). |

### Halt condition (telemetry)

The ability to loop indefinitely is inherent in Turing-complete systems (Halting Problem). The **language** provides the ability to loop; the **Runtime (Mwalimu)** provides governance. Implementation options include **gas metering** (e.g. for constrained VMs) or **watchdog timers** (e.g. embedded). The runtime may terminate or throttle a non-terminating program so that **Synthesis** can still complete for the rest of the system.

### Memory exhaustion (substrate strategy)

- **Standard mode:** `Orodha` (and other heap-backed types) expand until the OS or allocator denies the request. The runtime then triggers a **manifestation of failure**: **Abort**. The program does not continue; no `Tokeo` is returned for allocation failure in this mode.
- **Embedded / strict mode:** In constrained environments or when explicitly requested (e.g. via a “strict allocator” or `wazi`-adjacent APIs), the allocator can be configured so that operations like **Orodha::ongeza** return **Tokeo\<Tupu, Kosa\>** on allocation failure. Memory exhaustion becomes a handled **Signal** (explicit failure) rather than a fatal abort. See [Resolved Decisions](08-resolved-decisions.md) for the definitive rule.

---

## Memory strategy

- **Standard:** Ownership and borrowing by default with deterministic drop of owned values and resources. Allocation failure → **Abort** unless a stricter allocator is configured (see Execution-level friction above).
- **Managed / GC modules (optional):** Managed memory (e.g. reference counting or tracing GC) may be provided as opt-in library modules (such as `Kasha_GC<T>`), layered on top of the ownership model rather than replacing it.
- **Embedded:** Static allocation, packed structs, **no_std** core library; no GC. Allocation can be made explicit (e.g. **Orodha::ongeza** returns **Tokeo** when in strict/embedded mode).

---

## Development phases

| Phase | Name | Focus |
|-------|------|--------|
| **I** | Catalyst | Rust-based interpreter, terminal REPL, basic Pata. |
| **II** | Synthesis | LSP (Mwalimu), Wasm support, managed/GC module optimization (opt-in). |
| **III** | Resolution | Self-hosting (compiler written in Asili). |
| **IV** | Nguvu | LLVM backend, embedded targets, manual memory. |

---

## Feature–phase map

Concrete features per phase so each has a clear lifecycle; dependencies are respected (no async without runtime, no references without borrow checker).

| Phase | Focus (existing) | New / clarified features |
|-------|-------------------|---------------------------|
| **I — Catalyst** | Rust interpreter, REPL, basic Pata | Core complete: `umbo`, `shughuli ya`, Orodha index, Kamusi, Herufi, Jozi, `linganisha` (struct/Jozi). Optional: `vunja`, `endelea`, `lebo`. |
| **II — Synthesis** | LSP, Wasm, managed-memory modules (opt-in) | `linganisha`, `jaribu`/`?`, `Sifa`, `Jumla<T>`, `Pakiti`/`Moduli`, `#[jaribio]`, `#[sharti]`. Optional managed/GC module work can progress here without changing ownership-default semantics. Test runner honours `#[jaribio]`. |
| **III — Resolution** | Self-hosting | `Rejeo`, `Muda_wa_Kuishi` (borrow checker), `Mfululizo`, `Jozi`, `Seti`. Orodha strict mode (Tokeo on allocation failure). AST stable for macros. |
| **IV — Nguvu** | LLVM, embedded, manual memory | `tenda`, `njia`, `fungo` (concurrency). Then `sawia`/`subiri` once executor/runtime exists. `Kielelezo!`, `#[kiunganishi]`, FFI, `Kiashiria` in `wazi`. |

### Core complete (Phase I interpreter)

The Phase I interpreter is **core complete** when it supports: struct declaration with fields, struct literals and field access, impl blocks with method bodies and method dispatch on structs, Orodha index expression `expr[index]`, Kamusi via `kamusi_tupu` and methods `ingiza`/`pata`, Herufi (char) literals and casts, Jozi via `jozi(a,b)` and methods `kwanza`/`pili`, and pattern matching on structs and Jozi in `linganisha`. Borrowing remains copy semantics at runtime (no reference values). Mfululizo, Seti, Namba_Kuu, Namba_Sahihi, and full borrow checker are later phases.

**Stdlib and modules:** Code can use (1) **built-in stdlib** (e.g. hisabati, mfumo) implemented in the host and resolvable without disk, (2) the **in-tree stdlib surface** in **lib/std** (`.asi`/`.as`) for the same modules, and (3) **third-party modules** via **`[tegemezi]`** in `pata.toml`.

### Dependency notes

- **Async (`sawia`/`subiri`) and channels (`njia`):** Require a runtime/executor. Do not add async syntax before the VM or runtime can schedule tasks.
- **References and lifetimes:** Require a borrow checker. Do not add Rejeo/Muda_wa_Kuishi to the implementation before defining and implementing ownership/borrow rules.
- **Macros:** Require a stable AST and hygiene story; phase after parser and type checker are stable.
- **FFI:** Requires a stable ABI and type mapping; phase with or after LLVM/embedded target.

---

## Contribution workflow

- **Loop:** Msimbo (code) → Jaribio (`pata jaribu`) → Kiraka (patch) → Ukaguzi (review for Nguvu and Asili consistency) → Kuunganisha (merge by subsystem maintainer only).
- **Versioning:** **Mainline** (bleeding edge), **Stable** (production-ready), **LTS** (5+ years support).
- **Branch:** Mainline is **`asili-mainline`**.
- **Documentation:** Every new public `kazi` or `umbo` must have a documentation string. **CI check:** `pata thibitisha` fails if any public item lacks docs; the patch is rejected.

---

Previous: [Tooling and Ecosystem](06-tooling-and-ecosystem.md) | [Overview](../SPECIFICATION.md) | Next: [Resolved Decisions](08-resolved-decisions.md)
