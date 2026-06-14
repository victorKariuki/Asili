# Phase IIa Feature 2: Module Constants

**Goal:** Export `thabiti` declarations via `leta` imports

**Current State:**
- `ExportTable` has `constants: HashMap<String, ValueType>` but it's always empty
- Module-level constants are parsed but not tracked
- Constants cannot be imported/exported

**Desired Behavior:**
```asili
// math_lib.as
thabiti PI = 3.14159
thabiti E = 2.71828

// main.as
leta math_lib
chapisha(math_lib.PI)  // 3.14159
```

---

## Current Implementation Status

### 1. AST Structure
**File:** `core/parser/src/ast.rs`

`Module` struct currently has:
```rust
pub struct Module {
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub structs: Vec<StructDecl>,
    pub traits: Vec<TraitDecl>,
    pub impls: Vec<ImplDecl>,
    // MISSING: constants field
}
```

**Stmt::Let** structure:
```rust
Stmt::Let {
    mutable: bool,  // false = constant (thabiti)
    name: String,
    ty: Option<TypeExpr>,
    value: Expr,
    line: usize,
}
```

### 2. Resolver
**File:** `pata/cli/src/pipeline/resolve.rs`

`ExportTable`:
```rust
pub struct ExportTable {
    pub functions: HashMap<String, FnContract>,
    pub constants: HashMap<String, ValueType>,  // Currently always empty!
}
```

`build_export_table()`:
```rust
pub fn build_export_table(module: &Module) -> ExportTable {
    let mut functions = HashMap::new();
    let constants = HashMap::new();  // BUG: Never populated!
    for f in &module.functions {
        if f.is_public {
            // ... add to functions
        }
    }
    ExportTable { functions, constants }
}
```

### 3. Semantic Analyzer
**File:** `core/parser/src/semantic/analyzer.rs`

Constants are treated as mutable=false in Stmt::Let checks, but not exported.

---

## Implementation Plan (1-2 commits)

### Commit 1: Add Constants to Module AST + Export Table

**Step 1: Add constants field to Module**
- File: `core/parser/src/ast.rs`
- Add: `pub constants: Vec<Constant>`
- Define: `pub struct Constant { pub name: String, pub ty: TypeExpr, pub value: Expr, pub line: usize }`

**Step 2: Update parser to collect module-level constants**
- File: `core/parser/src/parse.rs`
- When parsing top-level statements:
  - If we see `thabiti` at module scope, collect into `Module::constants`
  - Skip adding to function body

**Step 3: Update build_export_table()**
- File: `pata/cli/src/pipeline/resolve.rs`
- For each constant in `module.constants`:
  - Get its type via `parse_value_type(&const.ty.name)`
  - Add to `ExportTable::constants` map

**Step 4: Initialize constants in Analyzer**
- File: `core/parser/src/semantic/analyzer.rs`
- In `check_function()`, add module constants to extern_constants
- Constants available in all functions

### Commit 2: Make Constants Available in Imported Modules

**Step 5: Import constants into scope**
- File: `pata/cli/src/pipeline/resolve.rs`
- When merging modules for evaluation, add constants to env
- Make constants accessible with dot notation: `module_name.CONSTANT`

**Step 6: Semantic check for constant references**
- File: `core/parser/src/semantic/analyzer.rs`
- Recognize `module.CONSTANT` patterns
- Check constant exists and is properly typed

---

## Testing Strategy

### Test 1: Parse module-level constants
```asili
thabiti PI = 3.14159
thabiti E = 2.71828

kazi kuu(h: Orodha<Neno>) -> Tupu {
    chapisha(PI)
}
```
**Expectation:** Parses without error

### Test 2: Export constants in ExportTable
```asili
// lib.as
thabiti MAGIC = 42.0

// main.as
leta lib
chapisha(lib.MAGIC)
```
**Expectation:** Constant is accessible

### Test 3: Constant type checking
```asili
thabiti X = "text"

kazi foo(n: Namba) -> Tupu {
    foo(X)  // Type mismatch
}
```
**Expectation:** SEM error - type mismatch

### Test 4: Constant is immutable
```asili
thabiti X = 5.0

kazi kuu(h: Orodha<Neno>) -> Tupu {
    X = 10.0  // Error: constants can't be reassigned
}
```
**Expectation:** SEM error - can't modify constant

### Test 5: Multiple constants
```asili
thabiti PI = 3.14159
thabiti E = 2.71828
thabiti TAU = 6.28318

kazi kuu(h: Orodha<Neno>) -> Tupu {
    chapisha(PI + E + TAU)
}
```
**Expectation:** All constants available and usable

---

## Key Differences from Variables

| Feature | Variable | Constant |
|---------|----------|----------|
| Declaration | `weka x = ...` | `thabiti x = ...` |
| Scope | Function/block local | Module-level |
| Export | No | Yes (via ExportTable) |
| Reassignment | Allowed (if mutable) | Never allowed |
| Type | Inferred or explicit | Required (for module constants) |

---

## Implementation Checklist

- [ ] Add `Constant` struct to AST
- [ ] Add `constants` field to `Module`
- [ ] Parse module-level `thabiti` into Module::constants
- [ ] Update `build_export_table()` to populate constants
- [ ] Add constants to extern_constants in semantic analyzer
- [ ] Support `module.CONSTANT` access pattern
- [ ] Type-check constant references
- [ ] Prevent reassignment of constants
- [ ] Write 5+ test cases
- [ ] Verify no Phase I regressions

---

## Complexity Assessment

**Low Risk:**
- ExportTable structure already exists
- Parser already handles `thabiti`
- Semantic analyzer already checks immutability

**Medium Risk:**
- Separating module-level constants from function-local ones
- Ensuring proper scoping in import merging

**Timeline:** 1-2 commits, ~2-3 hours

---

## Success Criteria

✅ Module-level `thabiti` declarations are parsed  
✅ Constants appear in ExportTable  
✅ Constants can be accessed via `module.CONSTANT`  
✅ Constant reassignment raises SEM error  
✅ Type checking works for constants  
✅ All Phase I tests still pass  

---

## Next: Feature 3

After Feature 2 lands, ready for:
- **Feature 3: Exception Handling** (4-5 commits)
  - Union types beyond `Tokeo<T>`
  - Error propagation patterns
  - Better error handling APIs
