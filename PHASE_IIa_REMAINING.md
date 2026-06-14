# Phase IIa Remaining Features (3-7): Implementation Plan

**Completed (2/7):**
- ✅ Drop Semantics (1 commit)
- ✅ Module Constants (1 commit)

**Remaining (5/7):**
- Feature 3: Exception Handling (4-5 commits)
- Feature 4: Enums (5-6 commits)
- Feature 5: Pattern Matching (4-5 commits)
- Feature 6: neno Module (3-4 commits)
- Feature 7: ASI Stub Parsing (1-2 commits)

**Total: ~18-22 commits | ~8-10 hours | 3-4 weeks**

---

## Feature 3: Exception Handling (4-5 commits)

**Goal:** Better error types + error propagation beyond `Tokeo<T>`

### What It Enables
- Proper error handling patterns
- Foundation for Enums (Feature 4)
- Better error propagation in user code

### Implementation Steps
1. **Commit 1:** Define error union type syntax
   - `Tokeo<T, E>` or `Result<T, E>` equivalent
   - Error enums with variants
   
2. **Commit 2:** Semantic analysis for error types
   - Type-check error handling
   - Error type compatibility
   
3. **Commit 3:** Error propagation operator
   - `?` operator equivalent in Asili
   - Early return on error
   
4. **Commit 4:** Standard error types
   - FileError, NetworkError, etc.
   - Standard error handling patterns

### Test Strategy
- 8+ tests covering error creation, propagation, handling

---

## Feature 4: Enums (5-6 commits)

**Goal:** Algebraic data types with variants

### What It Enables
- Discriminated unions
- Pattern matching foundation (Feature 5)
- Better data modeling

### Implementation Steps
1. **Commit 1:** Enum AST + parsing
   - `jenum Name { Variant1, Variant2(data) }`
   - Parser support for enum declarations
   
2. **Commit 2:** Enum evaluation
   - Runtime representation of enum values
   - Variant construction
   
3. **Commit 3:** Enum semantic analysis
   - Type-checking enum usage
   - Variant access validation
   
4. **Commit 4:** Enum methods (impl blocks)
   - Methods on enum types
   - Associated functions
   
5. **Commit 5:** Standard enums
   - Option<T> (Some/None)
   - Result<T, E> (Ok/Err)

### Test Strategy
- 10+ tests covering enum definition, construction, access, methods

---

## Feature 5: Pattern Matching (4-5 commits)

**Goal:** Destructuring in match arms, function params, for loops

### What It Enables
- Elegant enum destructuring
- Safer data access
- Foundation for advanced patterns

### Implementation Steps
1. **Commit 1:** Pattern destructuring in match
   - Match arms with destructuring
   - Variable binding from patterns
   
2. **Commit 2:** Patterns in function parameters
   - Function params with destructuring
   - Implicit binding
   
3. **Commit 3:** Patterns in for loops
   - for loop variable destructuring
   - Tuple unpacking
   
4. **Commit 4:** Exhaustiveness checking
   - Warn if patterns don't cover all cases
   - Suggest missing cases

### Test Strategy
- 8+ tests covering match patterns, function params, for loops

---

## Feature 6: neno Module (3-4 commits)

**Goal:** String manipulation methods

### What It Enables
- String operations without external functions
- Common text processing
- More ergonomic string handling

### Implementation Steps
1. **Commit 1:** String length + substring
   - `.urefu() -> Namba` — length
   - `.sehemu(start, end) -> Neno` — substring
   
2. **Commit 2:** String search + replace
   - `.contains(pattern) -> Ukweli` — search
   - `.kubadilisha(find, replace) -> Neno` — replace
   
3. **Commit 3:** String manipulation
   - `.nunua() -> Neno` — trim whitespace
   - `.jidhamirishaji(mode) -> Neno` — upper/lower case
   - `.sehemu_kwa(sep) -> Orodha<Neno>` — split

### Test Strategy
- 10+ tests covering all string methods

---

## Feature 7: ASI Stub Parsing (1-2 commits)

**Goal:** Multi-line function signatures

### What It Enables
- Readable long signatures
- Better code formatting
- Foundation for ASI improvements

### Implementation Steps
1. **Commit 1:** Allow newlines in param lists
   - Parse params across multiple lines
   - Maintain type checking
   
2. **Commit 2:** Allow newlines in return types
   - Optional improvements
   - Full multi-line support

### Test Strategy
- 3-4 tests covering multi-line sigs

---

## Dependency Graph

```
Feature 3: Exception Handling (foundational)
    ↓
Feature 4: Enums (uses error types)
    ↓
Feature 5: Pattern Matching (destructures enums)

Feature 6: neno Module (independent)

Feature 7: ASI Stub Parsing (independent)
```

---

## Recommended Execution Order

**Phase 1 (Day 1-2):** Features 3 + 4 + 5 (core language)
- Exception Handling (4-5 commits)
- Enums (5-6 commits)  
- Pattern Matching (4-5 commits)
- Total: 13-16 commits

**Phase 2 (Day 3):** Features 6 + 7 (polish)
- neno Module (3-4 commits)
- ASI Stub Parsing (1-2 commits)
- Total: 4-6 commits

**Total Phase IIa:** 7 + 13-16 + 4-6 = **24-29 commits total**

---

## Testing Strategy (All Features)

### Test Coverage Goals
- Exception Handling: 8+ tests
- Enums: 10+ tests
- Pattern Matching: 8+ tests
- neno Module: 10+ tests
- ASI Stub Parsing: 3-4 tests
- **Total: 39-42 tests**

### Regression Testing
- Phase I tests (10) - must still pass
- Drop Semantics tests (10) - must still pass
- Module Constants tests (10) - must still pass
- **Total: 30 baseline tests that must not regress**

---

## Success Criteria for Phase IIa

### Code Quality
- [ ] All 39-42 new tests passing
- [ ] All 30 baseline tests still passing
- [ ] No compiler warnings
- [ ] Clean git history (1 commit per feature step)

### Feature Completeness
- [ ] Drop Semantics: Use-after-drop detection working
- [ ] Module Constants: Constants exported in ExportTable
- [ ] Exception Handling: Error propagation patterns working
- [ ] Enums: Variant construction + access working
- [ ] Pattern Matching: Destructuring in all contexts
- [ ] neno Module: All string methods available
- [ ] ASI Stub Parsing: Multi-line signatures working

### Documentation
- [ ] PHASE_IIa_COMPLETE.md summarizing all features
- [ ] Code comments explaining complex sections
- [ ] Git commit messages documenting decisions

---

## Estimated Timeline

| Feature | Commits | Time | Status |
|---------|---------|------|--------|
| Drop Semantics | 1 | 1h | ✅ Done |
| Module Constants | 1 | 1h | ✅ Done |
| Exception Handling | 4-5 | 2-3h | ⏳ Next |
| Enums | 5-6 | 3-4h | ⏳ After |
| Pattern Matching | 4-5 | 2-3h | ⏳ After |
| neno Module | 3-4 | 1-2h | ⏳ After |
| ASI Stub Parsing | 1-2 | 0.5-1h | ⏳ Last |
| **TOTAL** | **18-23** | **10-15h** | ⏳ In Progress |

---

## Starting with Feature 3: Exception Handling

Ready to begin? Let's start with creating the implementation plan for Exception Handling.

Key decisions to make:
1. Should error types be built-in `Tokeo<T, E>` or user-defined via Enums?
2. Should we support `?` operator or equivalent?
3. Which standard error types to include (FileError, NetworkError, etc.)?
