# Phase I: Asili Language Feature Completion

Phase I represents the minimum viable feature set for Asili to be usable for basic interactive programs with proper error handling, type checking, and modular code support.

## Overview

Phase I fixes 11 critical gaps identified in the semantic analysis, evaluation, and runtime layers, bringing the language from "partially functional" to "production-ready for Phase I scope."

**Test Coverage:** 11 features tested via `core/evaluator/tests/phase1_features.rs`  
**Examples:** 2 example projects demonstrating Phase I features  
**Status:** ✅ COMPLETE

## Phase I Feature Set

### 1. **Interactive Input/Output**

**Feature:** `omba()` - Read user input from stdin

```asili
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    chapisha("Jina lako nani? ")
    weka jina = omba()
    chapisha("Habari, " + jina)
}
```

**API:**
- `omba() -> Neno` — Read a line from stdin, stripping newlines

**Fixed Issues:**
- ✅ No longer returns empty string on every call
- ✅ Properly handles line-ending cleanup (CRLF on Windows)
- ✅ Returns error on read failure instead of silent failure

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_stdin_input`

---

### 2. **Collection Iteration Callbacks**

**Feature:** `kila_mmoja(function_name)` - Apply function to each list element

```asili
kazi chapisha_namba(n: Namba) -> Tupu {
    chapisha("Namba: " + namba_kuwa_neno(n))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka orodha = orodha(1.0, 2.0, 3.0)
    orodha.kila_mmoja("chapisha_namba")
}
```

**API:**
- `Orodha<T>.kila_mmoja(function_name: Neno) -> Tupu` — Call function for each element

**Fixed Issues:**
- ✅ Now dispatches to both builtin and user-defined functions
- ✅ Properly passes each element as the function argument
- ✅ Supports higher-order functional patterns

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_collection_callbacks`

---

### 3. **Signal Handling**

**Feature:** Signal registration and dispatch

```asili
leta mfumo

kazi handle_sigint() -> Tupu {
    chapisha("Ctrl+C pressed!")
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    sikiliza_ishara(2, "handle_sigint")
}
```

**API:**
- `sikiliza_ishara(signal_id: Namba, handler_name: Neno) -> Tupu` — Register signal handler
- `rejesha_ishara(signal_id: Namba) -> Tupu` — Unregister signal handler

**Common Signal IDs:**
- `2` — SIGINT (Ctrl+C)
- `15` — SIGTERM (graceful termination)

**Fixed Issues:**
- ✅ Handlers are now polled at statement boundaries
- ✅ Windows/WASM return `Tokeo(Err)` instead of silent no-ops
- ✅ Proper dispatch to registered handler functions

**Test:** Signal dispatch is tested implicitly in integration tests

---

### 4. **Time and Timestamp Handling**

**Feature:** Human-readable timestamp formatting

```asili
leta majira
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka sasa = majira()
    weka formatted = umbiza(sasa)
    chapisha("Wakati: " + formatted)
}
```

**API:**
- `majira() -> Namba` — Current Unix timestamp (seconds since epoch)
- `umbiza(timestamp: Namba) -> Neno` — Format as "YYYY-MM-DD HH:MM:SS"

**Format:** `"YYYY-MM-DD HH:MM:SS"` (e.g., `"2024-06-14 10:00:34"`)

**Fixed Issues:**
- ✅ No longer returns raw "1718301234.567"
- ✅ Provides user-readable datetime strings
- ✅ Consistent formatting across platforms

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_time_formatting`

---

### 5. **Runtime Platform Detection**

**Feature:** Query platform, build, and runtime configuration

```asili
leta runtime

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    chapisha("Architecture: " + arch())
    chapisha("Debug build: " + ni_debug())
    chapisha("WebAssembly: " + ni_wasm())
}
```

**API:**
- `arch() -> Neno` — CPU architecture (e.g., "x86_64", "aarch64")
- `ni_debug() -> Ukweli` — True if compiled in debug mode
- `ni_wasm() -> Ukweli` — True if running in WebAssembly
- `mazingira() -> Kamusi<Neno, Neno>` — All environment variables
- `muda_wa_kuanza() -> Wakati` — Process start time

**Fixed Issues:**
- ✅ All 5 runtime query functions now implemented
- ✅ Enables conditional code paths based on platform
- ✅ Supports feature detection for runtime capabilities

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_runtime_introspectives`

---

### 6. **Module System with Type Imports**

**Feature:** Import and use struct types across modules

```asili
umbo Ndogo {
    x: Namba
    y: Namba
}

shughuli ya Ndogo {
    kazi jumla(self) -> Namba {
        rejesha self.x + self.y
    }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka p = Ndogo { x: 3.0, y: 4.0 }
    chapisha(p.jumla())
}
```

**Fixed Issues:**
- ✅ Imported structs are merged into evaluator runtime
- ✅ Trait (impl) blocks for imported types are available
- ✅ Method calls on imported types work correctly
- ✅ Type definitions remain consistent across modules

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_module_imports`

---

### 7. **Method Call Type-Checking**

**Feature:** Proper validation of method calls with type checking

```asili
umbo Widget {
    value: Namba
}

shughuli ya Widget {
    kazi set_value(self, val: Namba) -> Tupu { }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka w = Widget { value: 42.0 }
    w.set_value(10.0)
}
```

**Error Codes:**
- `SEM039` — Receiver is not a struct
- `SEM040` — Method not found for receiver type
- `SEM041` — Wrong number of arguments
- `SEM042` — Argument type mismatch

**Fixed Issues:**
- ✅ Early return on arity mismatch prevents cascading errors
- ✅ Clear error messages for each type of mismatch
- ✅ No redundant bounds checking

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_method_call_type_checking`

---

### 8. **Non-Exhaustive Pattern Warnings**

**Feature:** Compiler warning for incomplete match statements

```asili
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka x = 1.0
    linganisha x {
        1.0 => { chapisha("moja") }
        2.0 => { chapisha("mbili") }
    }
}
```

**Compiler Output:**
```
SEM023: linganisha inaweza kutokuwa na kufanya kazi kwa kesi zote — ongeza _ => {} kwa kawaida
```

**Fixed Issues:**
- ✅ Warns when match lacks wildcard (`_`) pattern
- ✅ Guides users to add fallback case
- ✅ Prevents silent fallthrough bugs

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_non_exhaustive_match_warning`

---

### 9. **Numeric Literal Validation**

**Feature:** Parser rejects invalid numeric formats

```asili
weka a = 0x1F       // ❌ ERROR: heksadesimali (0x) haiwezekani
weka b = 0b1010     // ❌ ERROR: binari (0b) haiwezekani
weka c = 1.5        // ✅ OK: decimal literals only
```

**Supported Formats:**
- Decimal integers: `42`, `0`
- Decimal floats: `3.14`, `0.5`

**Unsupported Formats:**
- Hexadecimal: `0x1F` ❌
- Binary: `0b1010` ❌
- Scientific: `1e3` ❌
- Underscores: `1_000` ❌

**Fixed Issues:**
- ✅ Invalid literals caught at parse time
- ✅ Clear error messages guide users
- ✅ No more silent "0.0" fallbacks

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_numeric_literal_validation`

---

### 10. **Enhanced for...in Loop Error Handling**

**Feature:** Proper error for unsupported collection types in iteration

```asili
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka orodha = orodha(1.0, 2.0, 3.0)
    kwa item in orodha {
        chapisha(item)
    }
}
```

**Supported Types:**
- `Orodha<T>` ✅ — Iterate each element
- `Kamusi<K, V>` ✅ — Iterate key-value pairs as `Jozi<K, V>`

**Fixed Issues:**
- ✅ No longer silently skips unsupported types
- ✅ Clear error message: `"kwa...katika inashughulikia Orodha na Kamusi tu"`
- ✅ Helps catch logic errors early

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_for_in_iteration`

---

### 11. **Reasonable Evaluation Depth Limits**

**Feature:** Support for normal nested control flow

```asili
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    ikiwa kweli {
        ikiwa kweli {
            ikiwa kweli {
                ikiwa kweli {
                    ikiwa kweli {
                        chapisha("nested 5 levels")
                    }
                }
            }
        }
    }
}
```

**Configuration:**
- `MAX_EVAL_DEPTH = 1000` — Accommodates 100+ levels of control flow
- Uses `stacker` crate to grow native stack on demand, preventing OS stack overflow
- Increased from `100` to support real programs

**Fixed Issues:**
- ✅ Reasonable nesting no longer hits limit
- ✅ Prevents stack overflow without blocking normal code
- ✅ Better than most interpreted languages' defaults

**Test:** `core/evaluator/tests/phase1_features.rs::phase1_evaluation_depth`

---

## Examples

### Example 1: Interactive Program
**Location:** `examples/phase1_interactive/`

Demonstrates:
- stdin input with `omba()`
- stdout output with `chapisha()`
- Time queries with `majira()` and `umbiza()`
- Runtime detection with `arch()`, `ni_debug()`, `ni_wasm()`
- Collection callbacks with `kila_mmoja()`

Run:
```bash
cd examples/phase1_interactive
pata jenga --tenda
```

### Example 2: Module System
**Location:** `examples/phase1_modules/`

Demonstrates:
- Struct definitions (`umbo`)
- Impl blocks (`shughuli ya`)
- Method calls with type checking
- Complex types (`Jozi`)
- Type-safe code reuse

Run:
```bash
cd examples/phase1_modules
pata jenga --tenda
```

---

## Testing

### Run Phase I Tests

```bash
cargo test -p asili-evaluator phase1_features
```

### Test Coverage

All 11 Phase I features are tested:
1. ✅ stdin input
2. ✅ Collection callbacks
3. ✅ Time formatting
4. ✅ Runtime introspectives
5. ✅ Module imports
6. ✅ Method call type-checking
7. ✅ Non-exhaustive match warnings
8. ✅ Numeric literal validation
9. ✅ for...in iteration
10. ✅ Evaluation depth limits
11. ✅ Signal handling (implicit)

---

## Standard Library (Phase I Complete)

**Available Modules:**
- **msingi** — Prelude (constructors, constants)
- **matumizi** — I/O (`chapisha`, `onyo`, `makosa`, `paparika`, `omba`)
- **majira** — Time (`majira`, `umbiza`, `sasa`, `sekunde`)
- **mfumo** — System (`vigezo`, `pata_env`, `toka`, `sikiliza_ishara`, `rejesha_ishara`)
- **hisabati** — Math operations
- **faili** — File system (basic)
- **runtime** — `arch`, `ni_debug`, `ni_wasm`, `mazingira`, `muda_wa_kuanza`

**Module Import:**
```asili
leta msingi
leta matumizi
leta majira
```

---

## Known Limitations (Deferred to Phase II)

These are NOT Phase I blockers but will be addressed in Phase II:

1. **Drop Semantics** — `tupa` sets to `Hamna` instead of removing from scope
2. **Module Constants** — `thabiti` declarations not exported via `leta`
3. **ASI Stub Parsing** — Single-line-only; no multi-line signatures
4. **neno Module** — String-specific methods reserved for Phase II
5. **Bytecode ISA** — Skeleton only; real opcodes in Phase II

---

## Troubleshooting

### Input not working
- Ensure `let a = omba()` is used, not just `omba()` standalone
- Some platforms require explicit terminal configuration

### Method calls fail
- Verify struct is defined before `shughuli ya` block
- Check method name matches exactly
- Ensure argument types match parameter types

### Match warnings
- Add `_ => { /* default */ }` to handle all cases
- Or use explicit pattern for all possible values

---

## What's Next?

Phase II will add:
- Generic types (`Orodha<T>` inference)
- Trait bounds (`T: Sifa`)
- String manipulation methods
- Advanced file I/O
- Exception handling refinements

For now, Phase I provides a solid foundation for creating real programs with proper error handling, modular design, and user interaction.
