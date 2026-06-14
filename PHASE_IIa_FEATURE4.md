# Phase IIa Feature 4: Enums (Algebraic Data Types)

**Goal:** Discriminated unions with variants

**Status:** Planning → Implementation Ready

---

## What It Enables

```asili
jenum Matokeo {
    Mafanikio(Namba),
    Kosa(Neno)
}

jenum Chaguo<T> {
    Kuna(T),
    Hamna
}

kazi foo() -> Matokeo {
    Mafanikio(42.0)
}

linganisha foo() {
    Mafanikio(n) => chapisha(n)
    Kosa(msg) => chapisha(msg)
}
```

---

## Implementation Plan (5-6 commits)

### Commit 1: Enum AST + Parsing
- Parse `jenum Name { Variant, Variant(Type) }`
- Store in Module::enums
- TypeExpr support for enum types

### Commit 2: Enum Semantic Analysis
- Type-check enum definitions
- Validate variant names are unique
- Resolve enum references

### Commit 3: Enum Value Representation
- Enum values in evaluator
- Variant construction
- Value equality

### Commit 4: Enum Methods (impl blocks)
- Methods on enum types
- Accessing enum variants

### Commit 5: Standard Enums
- Option<T> (Kuna, Hamna)
- Result<T, E> (Mafanikio, Kosa)

### Commit 6: Enum Exhaustiveness
- Warn if match doesn't cover all variants
- Suggest missing cases

---

## Key Design Points

**Variant Syntax:**
- Simple: `Mafanikio`
- With data: `Kosa(Neno)`
- Generic: `Kuna(T)`

**Construction:**
- `EnumName::Variant` for simple
- `EnumName::Variant(data)` for with data

**Pattern Matching:** (Feature 5)
- Destructure in match arms
- Variable binding from data

---

## Testing Strategy

- 10+ tests covering:
  - Enum definition parsing
  - Variant construction
  - Enum type checking
  - Methods on enums
  - Generic enums
  - Standard enums (Option, Result)

---

## Success Criteria

✅ Parse enum definitions  
✅ Create enum values  
✅ Type-check enum usage  
✅ Methods on enums  
✅ Standard enums work  
✅ No Phase I regressions  

---

## Next: Feature 5

Pattern Matching builds on Enums, adding destructuring in all contexts.
