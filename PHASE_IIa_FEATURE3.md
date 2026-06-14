# Phase IIa Feature 3: Exception Handling (Better Union Types)

**Goal:** Beyond `Tokeo<T>` — proper error types and propagation patterns

**Status:** Planning → Implementation Ready

---

## Current State: `Tokeo<T>` Limitations

```asili
// Current: single error type for all functions
kazi fungua_faili(jina: Neno) -> Tokeo<Neno> {
    // No way to distinguish error types
    // Error just returns generic error message
}

// Can't easily extract error details
linganisha fungua_faili("data.txt") {
    ok(data) => chapisha(data)
    kosa(msg) => chapisha("Error: " + msg)  // Too generic!
}
```

**Problems:**
1. No way to differentiate error types (file vs network vs parse)
2. No structured error data
3. Limited error recovery options
4. Error messages are just strings

---

## Desired Behavior: Better Union Types

```asili
// Define error types
jenum FileError {
    NotFound,
    PermissionDenied(Neno),
    IOError(Neno)
}

// Function returns specific error type
kazi fungua_faili(jina: Neno) -> Tokeo<Neno, FileError> {
    // Can return specific error variants
    rejesha kosa(FileError::NotFound)
}

// Pattern matching on specific errors
linganisha fungua_faili("data.txt") {
    ok(data) => chapisha(data)
    kosa(FileError::NotFound) => chapisha("File not found")
    kosa(FileError::PermissionDenied(reason)) => chapisha("Permission denied: " + reason)
    kosa(FileError::IOError(msg)) => chapisha("IO Error: " + msg)
}
```

---

## Implementation Plan (4-5 commits)

### Commit 1: Add Generic Error Type to `Tokeo<T>`

**Currently:** `Tokeo<T>` is hardcoded

**Change to:** `Tokeo<T, E>` with E defaulting to `Neno`

**Files to modify:**
- `core/parser/src/semantic/types.rs` — Type system
- `core/evaluator/src/value.rs` — Value representation
- `core/parser/src/ast.rs` — AST types

**Implementation:**
```rust
// Before
pub enum ValueType {
    Tokeo(Box<ValueType>),  // Only one type parameter
    // ...
}

// After
pub enum ValueType {
    Tokeo(Box<ValueType>, Box<ValueType>),  // Two type parameters: Ok and Error
    Chaguo(Box<ValueType>),  // Optional<T>
    // ...
}

// Backward compat: Tokeo<T> defaults to Tokeo<T, Neno>
```

**Tests:**
- Parse `Tokeo<T, E>` with two type parameters
- Backward compat: `Tokeo<T>` still works (defaults to `Tokeo<T, Neno>`)
- Type mismatch on error types detected

---

### Commit 2: Add `ok()` and `kosa()` Constructors

**Goal:** Create error/ok values with proper types

**Currently:**
```asili
rejesha Tokeo(ok: 42.0)  // Hardcoded
rejesha Tokeo(kosa: "error")  // Hardcoded
```

**Desired:**
```asili
rejesha ok(42.0)  // Function call
rejesha kosa(FileError::NotFound)  // Variant construction
```

**Files to modify:**
- `core/evaluator/src/builtins/msingi.rs` — Add `ok()`, `kosa()` functions
- `core/evaluator/src/eval/expr.rs` — Evaluate `ok(val)` and `kosa(val)`

**Implementation:**
```rust
// In msingi builtins
fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("ok".to_string(), Box::new(|args: &[Value]| {
        Ok(Value::Tokeo(Box::new(args[0].clone()), None))
    }));
    
    m.insert("kosa".to_string(), Box::new(|args: &[Value]| {
        Ok(Value::Tokeo(None, Box::new(args[0].clone())))
    }));
}
```

**Tests:**
- `ok(value)` creates success token
- `kosa(error)` creates error token
- Type checking validates error type
- Backward compat: `Tokeo(ok: value)` still works

---

### Commit 3: Pattern Matching on Error Types

**Goal:** Match specific error variants in error handler

**Build on:** Feature 5 (Pattern Matching) but implement basic matching first

**Implementation:**
```asili
linganisha fungua_faili("file.txt") {
    ok(data) => { /* handle success */ }
    kosa(err) => { /* handle generic error */ }
}
```

**Files to modify:**
- `core/parser/src/semantic/analyzer.rs` — Type-check pattern match
- `core/evaluator/src/eval/expr.rs` — Evaluate match with Tokeo variants

**Pattern Syntax:**
```
ok(variable)        → Extract ok value
kosa(variable)      → Extract error value
kosa(ErrorType)     → Match specific error type
kosa(ErrorType(...))→ Match with destructuring
```

**Tests:**
- Match on ok/kosa in Tokeo
- Extract values from patterns
- Type mismatch errors
- Exhaustiveness checking (both arms needed)

---

### Commit 4: Error Propagation Helper

**Goal:** Optional error propagation syntax (foundation for `?` operator in Phase II)

**Syntax option A: Early return function**
```asili
kazi foo() -> Tokeo<Neno, Error> {
    weka data = simu_fungua("file.txt")
    // data might be error - if so, return it
    weka data = jaribu(data)  // Unwrap or return error
    rejesha ok(data)
}
```

**Syntax option B: Try-catch equivalent**
```asili
kazi foo() -> Tokeo<Neno, Error> {
    jaribu linganisha fungua_faili("file.txt") {
        ok(data) => rejesha ok(data)
        kosa(err) => rejesha kosa(err)
    }
}
```

**Files to modify:**
- Add helper pattern or syntax sugar
- Evaluate into proper error handling

**Tests:**
- Error propagation works
- Type preservation through propagation
- Nested error contexts

---

### Commit 5: Standard Error Types (Optional)

**Goal:** Define common error enums in standard library

**Examples:**
```asili
jenum FileError {
    NotFound,
    PermissionDenied,
    IOError(Neno)
}

jenum ParseError {
    InvalidFormat,
    UnexpectedEOF,
    SyntaxError(Neno)
}

jenum NetworkError {
    ConnectionFailed,
    Timeout,
    DNSResolutionFailed
}
```

**Files to modify:**
- `core/evaluator/src/builtins/msingi.rs` — Define enums
- Documentation — Show how to use

**Tests:**
- Define custom errors
- Use in function return types
- Match on error variants

---

## Testing Strategy

### Test 1: Parse generic Tokeo
```asili
kazi foo() -> Tokeo<Namba, Neno> {
    rejesha ok(42.0)
}
```

### Test 2: Backward compat with Tokeo<T>
```asili
kazi foo() -> Tokeo<Namba> {
    rejesha ok(42.0)
}
```

### Test 3: `ok()` constructor
```asili
linganisha foo() {
    ok(value) => chapisha(value)
}
```

### Test 4: `kosa()` constructor
```asili
linganisha foo() {
    kosa(error) => chapisha(error)
}
```

### Test 5: Type mismatch on error
```asili
kazi foo() -> Tokeo<Namba, FileError> {
    rejesha kosa(ParseError::Invalid)  // Type error!
}
```

### Test 6: Exhaustiveness of match
```asili
linganisha foo() {
    ok(v) => v  // Missing kosa case - error
}
```

### Test 7: Error propagation
```asili
kazi foo() -> Tokeo<Namba, Error> {
    weka x = bar()  // If error, propagate
}
```

### Test 8: Nested Tokeo
```asili
kazi foo() -> Tokeo<Orodha<Namba>, Error> {
    rejesha ok(orodha(1.0, 2.0, 3.0))
}
```

---

## Key Design Decisions

### Decision 1: Generic Error Type
**Chosen:** `Tokeo<T, E>` with E defaulting to `Neno`
**Rationale:** 
- Flexible - users can define custom error types
- Compatible with Phase 4 (Enums)
- Gradual adoption (old code still works)

### Decision 2: Constructor Names
**Chosen:** `ok()` and `kosa()` functions
**Rationale:**
- Clear Swahili names
- Easy to distinguish from Tokeo enum
- Can be builtins

### Decision 3: Error Pattern Syntax
**Chosen:** Start simple, enhance in Feature 5
**Rationale:**
- Keep this feature focused
- Pattern matching full implementation in Feature 5
- Avoid feature creep

### Decision 4: Standard Errors
**Chosen:** Optional in Commit 5, can defer to Phase II if needed
**Rationale:**
- Foundation more important than standard types
- Can add FileError, etc. later
- Users can define their own

---

## Files Modified Summary

| File | Change | Reason |
|------|--------|--------|
| `core/parser/src/semantic/types.rs` | Add generic error param | Support `Tokeo<T, E>` |
| `core/evaluator/src/value.rs` | Update Value::Tokeo variant | Two-parameter union |
| `core/evaluator/src/builtins/msingi.rs` | Add `ok()`, `kosa()` | Constructors |
| `core/evaluator/src/eval/expr.rs` | Handle ok/kosa evaluation | Evaluate constructors |
| `core/parser/src/semantic/analyzer.rs` | Type-check Tokeo matches | Validation |
| New test file | `tests/exception_handling.rs` | 8+ tests |

---

## Complexity Assessment

**Low Risk:**
- Adding second type parameter is straightforward
- `ok()` and `kosa()` are simple builtins

**Medium Risk:**
- Type system changes affecting existing code
- Pattern matching integration

**High Risk:**
- Ensuring backward compat with `Tokeo<T>`
- Interactions with Feature 4 (Enums) design

---

## Success Criteria

✅ `Tokeo<T, E>` parses with two type params  
✅ `Tokeo<T>` still works (backward compat)  
✅ `ok()` and `kosa()` create proper values  
✅ Pattern matching works on ok/kosa  
✅ Type errors detected for mismatched error types  
✅ Exhaustiveness checking on Tokeo matches  
✅ All 8+ tests passing  
✅ No Phase I/II regressions  

---

## Next: Feature 4 (Enums)

Exception Handling sets up error types. Feature 4 (Enums) will:
- Define discriminated unions (like FileError)
- Support variant construction
- Enable destructuring in patterns
- Build on error type foundations
