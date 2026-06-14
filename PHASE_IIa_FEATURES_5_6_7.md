# Phase IIa Features 5-7: Quick Implementation Guides

---

## Feature 5: Pattern Matching (4-5 commits)

**Goal:** Destructuring in all contexts

**Depends on:** Feature 4 (Enums)

### Implementation Steps

1. **Pattern Parser** (1 commit)
   - Parse patterns in match arms: `Variant(x, y) => ...`
   - Parse wildcard: `_ => ...`
   - Parse literals: `42 => ...`

2. **Match Evaluation** (2 commits)
   - Destructure enum variants
   - Bind variables from patterns
   - Type-check pattern types

3. **Function Parameters** (1 commit)
   - Destructure in params: `kazi foo(Jozi(x, y)) { ... }`

4. **For Loops** (1 commit)
   - Destructure in for: `kwa Jozi(x, y) katika lista { ... }`

### Testing
- 8+ tests covering match patterns, param destructuring, for loops

### Success Criteria
✅ Patterns destructure enum variants  
✅ Variables bound from pattern data  
✅ Type checking works  
✅ Works in match, params, for loops  

---

## Feature 6: neno Module (3-4 commits)

**Goal:** String methods

**Independent** - no dependencies

### Implementation Steps

1. **String Length + Substring** (1 commit)
   - `.urefu() -> Namba`
   - `.sehemu(start, end) -> Neno`

2. **Search + Replace** (1 commit)
   - `.contains(pattern) -> Ukweli`
   - `.kubadilisha(find, replace) -> Neno`

3. **Case + Whitespace** (1 commit)
   - `.nunua() -> Neno`
   - `.jidhamirishaji(mode) -> Neno`
   - `.sehemu_kwa(sep) -> Orodha<Neno>`

4. **Tests** (1 commit)
   - 10+ tests for all string methods

### Testing
- Test each method with various inputs
- Edge cases (empty strings, no match, etc.)

### Success Criteria
✅ All string methods available  
✅ Return correct types  
✅ Handle edge cases  

---

## Feature 7: ASI Stub Parsing (1-2 commits)

**Goal:** Multi-line function signatures

**Independent** - no dependencies

### Implementation Steps

1. **Multi-line Params** (1 commit)
   - Allow newlines in parameter lists
   - `kazi foo(\n  x: Namba,\n  y: Namba\n) -> Namba`

2. **Multi-line Return Type** (optional, 1 commit)
   - Allow newlines in return types
   - Full formatting support

### Testing
- 3-4 tests for multi-line signatures

### Success Criteria
✅ Parse multi-line params  
✅ Type checking works  
✅ Functions work normally  

---

## Recommended Execution Order

### Day 1: Features 4-5 (Core Language)
```
Feature 4 (Enums):
  Commit 1: AST + Parsing
  Commit 2: Semantic Analysis
  Commit 3: Value Representation
  Commit 4: Methods
  Commit 5: Standard Enums
  Commit 6: Exhaustiveness
  Tests: 10+

Feature 5 (Pattern Matching):
  Commit 1: Pattern Parser
  Commit 2: Match Evaluation
  Commit 3: Function Params
  Commit 4: For Loops
  Tests: 8+
```

### Day 2: Features 6-7 (Polish)
```
Feature 6 (neno Module):
  Commit 1: Length + Substring
  Commit 2: Search + Replace
  Commit 3: Case + Whitespace
  Commit 4: Tests

Feature 7 (ASI Parsing):
  Commit 1: Multi-line Params
  Commit 2: Multi-line Return (optional)
  Tests: 3-4
```

---

## Quick Reference

| Feature | Commits | Time | Type | Blocks |
|---------|---------|------|------|--------|
| 4. Enums | 5-6 | 3-4h | Core | Feature 5 |
| 5. Pattern Match | 4-5 | 2-3h | Core | Nothing |
| 6. neno Module | 3-4 | 1-2h | Stdlib | Nothing |
| 7. ASI Parsing | 1-2 | 0.5-1h | Polish | Nothing |

---

## Parallel Work Opportunities

**Can be done in parallel:**
- Features 6 and 7 (independent)
- Early tests for Feature 5 while Feature 4 is being implemented

**Must be sequential:**
- Feature 4 → Feature 5 (dependency)

---

## Git Strategy

- One commit per implementation step
- Tests in same commit as implementation
- Clear messages explaining each step
- All Phase I tests verified after each major feature

---

## Success Criteria for Phase IIa Complete

✅ Enums fully implemented  
✅ Pattern matching works in all contexts  
✅ All string methods available  
✅ Multi-line signatures parse  
✅ 40+ new tests passing  
✅ All Phase I tests still passing  
✅ No compiler warnings  
✅ Clean git history  

---

## Estimated Totals

- **Commits:** 13-17
- **Test Cases:** 40+
- **Time:** 6-10 hours
- **Code Added:** 1000+ lines
