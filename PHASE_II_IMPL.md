# Phase II Implementation Guide

## IIa Feature 1: Drop Semantics

**Goal:** Make `tupa x` remove variable from scope entirely, not just set to `Hamna`

**Current Behavior:**
```asili
weka x = 5.0
tupa x
chapisha(x)  // Returns Hamna (bug!)
```

**Desired Behavior:**
```asili
weka x = 5.0
tupa x
chapisha(x)  // ERROR: SEM027 "x" haijaelezwa
```

---

## Understanding Current Implementation

### 1. AST Definition
**File:** `core/parser/src/ast.rs`
```rust
Stmt::Drop {
    name: String,    // Variable name
    line: usize,     // Line number
}
```

### 2. Parsing
**File:** `core/parser/src/parse.rs`
```rust
if self.match_tok("tupa") {
    let ident = self.consume_ident("PAR030", "tupa inahitaji jina")?;
    // Creates Stmt::Drop { name: ident, line }
}
```

### 3. Current Evaluation
**File:** `core/evaluator/src/eval/stmt.rs`
```rust
Stmt::Drop { name, .. } => {
    rt.env.set(name, Value::Hamna);  // BUG: Just sets to null
    Ok(EvalOut::Next)
}
```

### 4. Environment Structure
**File:** `core/evaluator/src/env.rs`
```rust
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,  // Stack of scopes
}
```

---

## Implementation Plan

### Step 1: Add `drop()` Method to Env
**File:** `core/evaluator/src/env.rs`

Add new public method:
```rust
pub fn drop(&mut self, name: &str) -> bool {
    // Search from innermost to outermost scope
    for scope in self.scopes.iter_mut().rev() {
        if scope.contains_key(name) {
            scope.remove(name);
            return true;  // Found and removed
        }
    }
    false  // Variable not found
}
```

**Why:** Cleanly removes variable from its scope, not setting to Hamna

---

### Step 2: Update Evaluation
**File:** `core/evaluator/src/eval/stmt.rs`

Replace current Drop handling:
```rust
Stmt::Drop { name, .. } => {
    // Try to drop the variable
    if !rt.env.drop(name) {
        // Variable doesn't exist - this should have been caught at semantic analysis
        // but if we hit it, return error
        return Err(Diagnostic::new(
            "EVM999",
            format!("tupa: {} haijaelezwa", name),
        ));
    }
    Ok(EvalOut::Next)
}
```

**Why:** Actually remove the variable instead of silently setting to null

---

### Step 3: Semantic Analysis (Already Correct!)
**File:** `core/parser/src/semantic/analyzer.rs`

Current code already checks that variable exists:
```rust
Stmt::Drop { name, line } => {
    if !self.is_defined(name) {
        self.error(SEM027, format!("tupa inatumia jina lisilojulikana: {name}"), Some(*line));
    }
}
```

**No changes needed here** — semantic check is correct. It prevents dropping undefined variables.

---

### Step 4: Handle Use-After-Drop
**File:** `core/parser/src/semantic/analyzer.rs`

Current semantic analyzer only checks if variable is "defined", not if it's been dropped.

We need a **"dropped variables" tracker**:

```rust
pub struct Analyzer {
    // ... existing fields ...
    dropped_vars: HashSet<String>,  // Track dropped variables in current scope
}

// When analyzing Drop statement:
Stmt::Drop { name, line } => {
    if !self.is_defined(name) {
        self.error(SEM027, ...);
    } else {
        // Track that this variable is now dropped
        self.dropped_vars.insert(name.clone());
    }
}

// When analyzing variable reference:
Expr::Var { name } => {
    if self.dropped_vars.contains(name) {
        // ERROR: Reading dropped variable
        self.error(SEM028, format!("{} tupwa na haipaswi kutumiwa", name), ...);
    }
    // ... existing checks ...
}
```

**Why:** Prevent use-after-drop at compile time

**Complexity:** Scope management — when scope exits, clear dropped_vars tracking

---

### Step 5: Define Error Code SEM028
**File:** `core/evaluator/src/eval/stmt.rs` or diagnostic definitions

```rust
SEM028: "{name} tupwa na haipaswi kutumiwa"
// English: "dropped variable used after drop"
```

---

## Testing Strategy

### Test 1: Basic Drop
```asili
kazi test_basic_drop() -> Tupu {
    weka x = 5.0
    tupa x
    // Next line should error: x not found
}
```
**Expectation:** SEM028 "x tupwa na haipaswi kutumiwa"

### Test 2: Drop Undefined Variable
```asili
kazi test_drop_undefined() -> Tupu {
    tupa y  // y was never defined
}
```
**Expectation:** SEM027 "y haijaelezwa"

### Test 3: Drop in Function
```asili
kazi test_drop_in_func() -> Tupu {
    weka buffer = "data"
    // ... use buffer ...
    tupa buffer  // Cleanup
    // Can't use buffer here
}
```
**Expectation:** Compiles, buffer cannot be used after drop

### Test 4: Drop in Nested Scope
```asili
kazi test_drop_nested() -> Tupu {
    weka x = 5.0
    ikiwa kweli {
        tupa x
        // x is dropped in inner scope
    }
    // x is still available in outer scope (shadow handling)
}
```
**Expectation:** TBD — depends on scope semantics

### Test 5: Drop and Variable Shadowing
```asili
kazi test_drop_shadow() -> Tupu {
    weka x = 5.0
    tupa x
    weka x = 10.0  // New x in same scope?
    chapisha(x)    // Should work
}
```
**Expectation:** Allowed (new binding after drop)

---

## Edge Cases to Consider

1. **Dropping in different scopes**
   - Drop in function, try to use in parent scope?
   - Should be caught by scope rules

2. **Dropping function parameters**
   ```asili
   kazi foo(x: Namba) -> Tupu {
       tupa x
       chapisha(x)  // ERROR
   }
   ```
   Should work — parameters are variables

3. **Dropping and looping**
   ```asili
   kwa i katika orodha {
       tupa i  // Should this work?
   }
   ```
   Need to decide: loop variables are implicitly redefined each iteration?

4. **Multiple drops of same variable**
   ```asili
   weka x = 5.0
   tupa x
   tupa x  // ERROR: x already dropped
   ```
   Evaluator will catch this (drop returns false)

---

## Implementation Checklist

- [ ] **Env::drop()** — Add method to remove from scope (env.rs)
- [ ] **Stmt::Drop evaluation** — Call drop() instead of set() (stmt.rs)
- [ ] **dropped_vars tracking** — Add HashSet<String> to Analyzer
- [ ] **Use-after-drop check** — Error on dropped variable reference
- [ ] **SEM028 error code** — Define new diagnostic
- [ ] **Scope management** — Clear dropped_vars on scope exit
- [ ] **Tests** — 5+ comprehensive test cases
- [ ] **Documentation** — Update PHASE_II.md with learnings

---

## Risk Assessment

**Low Risk:**
- Env::drop() is a simple new method
- Drop evaluation is straightforward

**Medium Risk:**
- Use-after-drop tracking requires careful scope management
- Need to ensure dropped_vars doesn't bleed across scope boundaries

**Not Needed (Future):**
- Borrow checking (full Rust-style ownership)
- Move semantics
- Lifetime annotations

---

## Commit Strategy

**Commit 1:** Env::drop() method + evaluation update
```
feat(drop): implement variable removal via tupa statement

- Add Env::drop() to remove variables from scope
- Update Stmt::Drop evaluation to call drop() instead of set()
- Variables are now truly dropped, not just set to Hamna
- Use-after-drop still allowed at runtime (caught next)
```

**Commit 2:** Use-after-drop semantic checking
```
feat(drop): add SEM028 for use-after-drop detection

- Track dropped variables in semantic analyzer
- Emit SEM028 error when dropped variable is referenced
- Scope-aware: dropped_vars clears on scope exit
- Prevents subtle bugs from using dropped resources
```

---

## Success Criteria

✅ `tupa x` removes x from environment  
✅ Reading x after `tupa x` causes compile-time error (SEM028)  
✅ Dropping undefined variable causes SEM027  
✅ All tests pass  
✅ No regressions in Phase I tests  
✅ Code is well-commented  

---

## Next: Feature 2 (Module Constants)

After Drop Semantics merged, move to Module Constants.
