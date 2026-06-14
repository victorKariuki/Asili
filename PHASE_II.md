# Phase II: Complete Language & Ecosystem (30 Features)

**Status:** Planning → Implementation  
**Timeline:** 6-8 months (aggressive but achievable)  
**Commits:** ~101-123  
**Target:** Production-ready language with professional tooling

---

## Phase IIa: Correctness Foundation (7 features, 25-30 commits)

**CRITICAL PATH - Blocks everything else**

### 1. Drop Semantics (2-3 commits)
**What:** `tupa` removes variable from scope entirely (not just set to `Hamna`)  
**Where:** 
- `core/evaluator/src/eval/stmt.rs` — Implement `Stmt::Tupa` evaluation
- `core/parser/src/semantic/analyzer.rs` — Type-check `tupa` statements

**Impact:** Enables resource cleanup, proper garbage collection patterns

**Tests:** Verify variable is inaccessible after `tupa`

---

### 2. Exception Handling (4-5 commits)
**What:** Better error types, error propagation  
**Where:**
- `core/evaluator/src/value.rs` — Add better error enum (move beyond `Tokeo<T>`)
- `core/evaluator/src/eval/expr.rs` — Implement error propagation
- `core/parser/src/semantic/analyzer.rs` — Type-check error flows

**Depends on:** Drop Semantics (resource cleanup in error paths)

**Impact:** Users can write robust error handling without nested `Tokeo` hell

---

### 3. Module Constants (1-2 commits)
**What:** `thabiti` declarations export via `leta` imports  
**Where:**
- `pata/cli/src/pipeline/resolve.rs` — Export constants in module table
- `core/parser/src/semantic/analyzer.rs` — Track constants in scope

**Impact:** Configuration, shared constants across modules

---

### 4. Enums (5-6 commits)
**What:** Algebraic data types with variants  
**Syntax:**
```asili
jenum Matokeo {
  Mafanikio(Namba),
  Kosa(Neno)
}
```
**Where:**
- `core/lexer/src/lib.rs` — Add `jenum` keyword
- `core/parser/src/parse.rs` — Parse enum definitions
- `core/parser/src/ast.rs` — Add Enum AST node
- `core/evaluator/src/value.rs` — Enum value type
- `core/evaluator/src/eval/expr.rs` — Construct enum variants
- `core/parser/src/semantic/analyzer.rs` — Type-check variants

**Depends on:** Exception Handling (enums used for errors)

**Impact:** Proper discriminated unions, foundation for pattern matching

---

### 5. Full Pattern Matching (4-5 commits)
**What:** Destructuring in function params, for loops, match arms  
**Syntax:**
```asili
linganisha x {
  Matokeo::Mafanikio(n) => { chapisha(n) }
  Matokeo::Kosa(msg) => { chapisha(msg) }
}
```
**Where:**
- `core/parser/src/semantic/analyzer.rs` — Type-check destructuring
- `core/evaluator/src/eval/expr.rs` — Evaluate pattern matches
- `core/evaluator/src/eval/stmt.rs` — Destructure in for loops, function params

**Depends on:** Enums

**Impact:** Ergonomic code, leverages enum types

---

### 6. neno Module: String Methods (3-4 commits)
**What:** Methods on `Neno` type  
**Methods:**
- `.urefu() -> Namba` — string length
- `.sehemu(start: Namba, end: Namba) -> Neno` — substring
- `.kubadilisha(find: Neno, replace: Neno) -> Neno` — string replacement
- `.nunua() -> Neno` — trim whitespace
- `.jidhamirishaji(mode: Neno) -> Neno` — upper/lower case
- `.sehemu_kwa(sep: Neno) -> Orodha<Neno>` — split

**Where:**
- `core/evaluator/src/builtins/neno.rs` — NEW module (parallel-safe)
- `core/evaluator/src/eval/expr.rs` — Method call dispatch for Neno

**Impact:** String manipulation without external functions

---

### 7. ASI Stub Parsing (1-2 commits)
**What:** Multi-line function signatures  
**Syntax:**
```asili
kazi kuu(
  jina: Neno,
  umri: Namba
) -> Tupu {
  ...
}
```
**Where:**
- `core/parser/src/parse.rs` — Allow newlines in param lists

**Impact:** Readable long function signatures

---

**IIa Success Criteria:**
- [ ] All 7 features implemented
- [ ] 10+ comprehensive tests per feature
- [ ] No regressions in Phase I features
- [ ] Documentation updated

---

## Phase IIb: Type System & Ergonomics (7 features, 22-26 commits)

**Depends on:** IIa complete  
**Can run parallel with:** IIc

### 8. Generic Type Inference (3-4 commits)
**What:** Infer `Orodha<T>` and `Kamusi<K,V>` types from context  
**Before:**
```asili
weka lista: Orodha<Namba> = orodha(1.0, 2.0, 3.0)
```
**After:**
```asili
weka lista = orodha(1.0, 2.0, 3.0)  // Type inferred: Orodha<Namba>
```

**Where:**
- `core/parser/src/semantic/analyzer.rs` — Implement type inference

---

### 9. Trait Bounds (3-4 commits)
**What:** Generic constraints with `T: Sifa`  
**Syntax:**
```asili
kazi jumla<T: Sifa>(lista: Orodha<T>) -> T {
  // T must implement Sifa (addable, orderable, etc.)
}
```
**Where:**
- `core/parser/src/ast.rs` — Add trait bounds to generics
- `core/parser/src/semantic/analyzer.rs` — Check trait implementations

**Depends on:** Generic Type Inference

---

### 10. Type Aliases (1 commit)
**What:** Define custom type names  
**Syntax:**
```asili
taarifa Msimbo = Neno
taarifa Hesabu = Namba
```
**Where:**
- `core/parser/src/parse.rs` — Parse `taarifa` statements
- `core/parser/src/semantic/analyzer.rs` — Resolve aliases

---

### 11. Closures (6-7 commits)
**What:** First-class functions with capture  
**Syntax:**
```asili
weka x = 5.0
weka adder = kazi(y) { rejesha x + y }
chapisha(adder(3.0))  // 8.0
```
**Where:**
- `core/parser/src/ast.rs` — Add closure expression
- `core/parser/src/parse.rs` — Parse closure syntax
- `core/evaluator/src/value.rs` — Closure value type
- `core/evaluator/src/eval/expr.rs` — Evaluate closures, capture variables
- `core/parser/src/semantic/analyzer.rs` — Type-check closures

**Depends on:** Drop Semantics (lifetime tracking)

**Complex:** Requires scope tracking and variable capture

---

### 12. Operator Overloading (3-4 commits)
**What:** Custom `+`, `-`, `*`, `/` on user types  
**Syntax:**
```asili
shughuli ya Widget {
  kazi +(self: Widget, other: Widget) -> Widget {
    rejesha Widget { value: self.value + other.value }
  }
}

weka w1 = Widget { value: 5.0 }
weka w2 = Widget { value: 3.0 }
weka w3 = w1 + w2  // Calls overloaded + method
```
**Where:**
- `core/parser/src/parse.rs` — Parse operator method definitions
- `core/evaluator/src/eval/expr.rs` — Dispatch binary ops to methods
- `core/parser/src/semantic/analyzer.rs` — Type-check operator calls

---

### 13. String Interpolation (2-3 commits)
**What:** Inline variable substitution in strings  
**Syntax:**
```asili
weka jina = "Asili"
chapisha("Habari, {jina}!")  // "Habari, Asili!"
```
**Where:**
- `core/lexer/src/lib.rs` — Tokenize `{expr}` in strings
- `core/parser/src/parse.rs` — Parse string interpolation expressions
- `core/evaluator/src/eval/expr.rs` — Evaluate interpolations at runtime

---

### 14. Format Strings (1-2 commits)
**What:** Debug/pretty-print format specifiers  
**Syntax:**
```asili
chapisha("{:?}", value)      // Debug format
chapisha("{:#?}", value)     // Pretty-print format
```
**Where:**
- Extend `chapisha()` builtin in `core/evaluator/src/builtins/matumizi.rs`

---

**IIb Success Criteria:**
- [ ] All 7 features implemented
- [ ] Generics properly inferred
- [ ] Closures capture variables correctly
- [ ] Operator overloading works for all binary ops

---

## Phase IIc: Standard Library Expansion (5 features, 18-22 commits)

**Depends on:** IIa complete  
**Can run parallel with:** IIb

### 15. Advanced File I/O (4-5 commits)
**What:** File operations beyond skeleton  
**API:**
- `kata(path: Neno) -> Tokeo<Neno>` — Read file contents
- `andika(path: Neno, content: Neno) -> Tokeo<Tupu>` — Write file
- `orodha_faili(dir: Neno) -> Tokeo<Orodha<Neno>>` — List directory
- `je_na_faili(path: Neno) -> Ukweli` — File exists
- `je_saraka(path: Neno) -> Ukweli` — Is directory

**Where:**
- `core/evaluator/src/builtins/faili.rs` — Implement file ops
- Wraps `std::fs` module

**Depends on:** Exception Handling (error returns)

---

### 16. JSON Support (4-5 commits)
**What:** Parse and stringify JSON  
**API:**
- `JSON.kusomeka(text: Neno) -> Tokeo<Kamusi>` — Parse JSON
- `JSON.kuandika(obj: Kamusi) -> Neno` — Stringify to JSON
- New type: `JSON` (alias for nested structures)

**Where:**
- `core/evaluator/src/builtins/json.rs` — NEW module
- Wraps `serde_json` crate

---

### 17. Regex Module (4-5 commits)
**What:** Pattern matching on strings  
**API:**
- `Regex.chunguza(pattern: Neno, text: Neno) -> Ukweli` — Match
- `Regex.sehemu(pattern: Neno, text: Neno) -> Orodha<Neno>` — Split
- `Regex.badilisha(pattern: Neno, replacement: Neno, text: Neno) -> Neno` — Replace

**Where:**
- `core/evaluator/src/builtins/regex.rs` — NEW module
- Wraps `regex` crate

---

### 18. HTTP Client (3-4 commits)
**What:** Basic HTTP requests  
**API:**
- `HTTP.soma(url: Neno) -> Tokeo<Neno>` — GET request
- `HTTP.tuma(url: Neno, data: Neno) -> Tokeo<Neno>` — POST request
- `HTTP.vichwa(url: Neno) -> Tokeo<Kamusi<Neno, Neno>>` — GET headers

**Where:**
- `core/evaluator/src/builtins/http.rs` — NEW module
- Wraps `reqwest` or `ureq` crate

---

### 19. Database Drivers (3-4 commits)
**What:** SQLite + PostgreSQL support  
**API (SQLite):**
- `DB.fungua(":memory:") -> Tokeo<Kiini>` — Open database
- `kiini.fanya(sql: Neno) -> Tokeo<Tupu>` — Execute SQL
- `kiini.soma(sql: Neno) -> Tokeo<Orodha<Kamusi>>` — Query rows

**API (PostgreSQL):**
- Similar, with connection string

**Where:**
- `core/evaluator/src/builtins/database.rs` — NEW module
- Wraps `rusqlite` (SQLite) and `postgres` (PostgreSQL) crates

---

**IIc Success Criteria:**
- [ ] File I/O covers common patterns
- [ ] JSON works with nested structures
- [ ] Regex patterns match real-world needs
- [ ] HTTP basic auth + headers supported
- [ ] SQLite + PostgreSQL CRUD working

---

## Phase IId: Testing & Tooling (4 features, 10-12 commits)

**Depends on:** IIa (parser/semantic) complete  
**Can run parallel with:** IIe

### 20. Testing Framework (3-4 commits)
**What:** Built-in test runner and assertions  
**Syntax:**
```asili
#[jaribio]
kazi jaribio_jumla() -> Tupu {
  thibitisha(1.0 + 1.0 == 2.0)
  thibitisha_sawa("habari", "habari")
}
```
**Command:** `pata jaribio` — Run all tests

**Where:**
- `core/parser/src/parse.rs` — Recognize `#[jaribio]` attribute
- `pata/cli/src/main.rs` — Add `pata jaribio` subcommand
- `core/evaluator/src/builtins/testing.rs` — `thibitisha()`, `thibitisha_sawa()`

**Features:**
- Parallel test execution
- Summary: passed/failed/skipped
- Detailed failure messages

---

### 21. Code Formatter (2-3 commits)
**What:** Auto-format `.as` files  
**Command:** `pata sambaza [file.as]`

**Where:**
- `pata/cli/src/main.rs` — Add `pata sambaza` subcommand
- `pata/cli/src/formatter.rs` — NEW module
- Uses AST walking (not regex)

**Rules:**
- 4-space indentation
- One statement per line
- Spacing around operators
- Consistent brace placement

---

### 22. Linter (2-3 commits)
**What:** Style & logic error checking  
**Command:** `pata angalia [file.as]`

**Checks:**
- Unused variables warning
- Unreachable code warning
- Function never returns warning
- Mismatched indentation warning

**Where:**
- `pata/cli/src/main.rs` — Add `pata angalia` subcommand
- `pata/cli/src/linter.rs` — NEW module
- Uses semantic analysis results

---

### 23. Doc Generator (2-3 commits)
**What:** Generate HTML documentation  
**Command:** `pata hati` — Generate docs from code

**Features:**
- Parse code comments (`// comment`, `/* block */`)
- Generate HTML for each module/function/struct
- Create index page

**Where:**
- `pata/cli/src/main.rs` — Add `pata hati` subcommand
- `pata/cli/src/doc_generator.rs` — NEW module

---

**IId Success Criteria:**
- [ ] Tests run in parallel successfully
- [ ] Formatter produces idiomatic code
- [ ] Linter catches real issues
- [ ] Docs generate valid HTML

---

## Phase IIe: Developer Experience (3 features, 11-13 commits)

**Depends on:** IIa + evaluator complete  
**Can run parallel with:** IId

### 24. REPL (4-5 commits)
**What:** Interactive shell for experimentation  
**Command:** `pata karibu`

**Features:**
- Parse + evaluate each line
- Multi-line input (e.g., function definitions)
- Command history
- Inspect variable values
- `:help`, `:exit` commands

**Where:**
- `pata/cli/src/main.rs` — Add `pata karibu` subcommand
- `pata/cli/src/repl.rs` — NEW module
- Uses lineeditor crate for history

---

### 25. Package Manager (4-5 commits)
**What:** Publish/download modules from registry  
**Commands:**
- `pata taka faili jina/moduli` — Download module
- `pata chapisha` — Publish module to registry
- `pata kuomba moduli` — List available modules

**Where:**
- `pata/cli/src/main.rs` — Add new subcommands
- `pata/cli/src/registry.rs` — NEW module (registry client)
- Improves on current manual system

**Registry:**
- Central server storing module metadata + source
- Semantic versioning (1.0.0, 1.2.3, etc.)
- Dependency resolution (transitive)

---

### 26. Debugger (3-4 commits)
**What:** Step-through debugging  
**Command:** `pata fungua [file.as]`

**Features:**
- Breakpoints (by line number)
- Step over/into/out
- Print variable values
- Watch expressions
- Call stack inspection

**Where:**
- `pata/cli/src/main.rs` — Add `pata fungua` subcommand
- `core/evaluator/src/debugger.rs` — NEW module
- May integrate with GDB via mi (machine interface)

---

**IIe Success Criteria:**
- [ ] REPL feels interactive and responsive
- [ ] Package download/publish works end-to-end
- [ ] Debugger stops at breakpoints
- [ ] Variable inspection is accurate

---

## Phase IIf: Virtual Machine (1 feature, 15-20 commits)

**Depends on:** IIa + IIb complete  
**Final Polish:** Performance + cleanup

### 27. Bytecode ISA (15-20 commits)
**What:** Real opcodes instead of tree-walking interpreter  
**Opcodes:**
```
LOAD_CONST <idx>       // Load constant
LOAD_VAR <name>        // Load variable
STORE_VAR <name>       // Store variable
CALL <name> <argc>     // Call function
RETURN                 // Return from function
JUMP <offset>          // Unconditional jump
JUMP_IF_FALSE <offset> // Jump if condition false
BUILD_LIST <count>     // Create list from stack
BUILD_MAP <count>      // Create map from stack
BINARY_ADD             // Pop 2, push sum
...
```

**Architecture:**
1. Compiler: AST → bytecode (new phase)
2. VM: Execute bytecode (replaces evaluator)
3. Performance: 2-3x faster

**Where:**
- `core/bytecode/src/lib.rs` — NEW crate
- `core/bytecode/src/compiler.rs` — AST to bytecode
- `core/bytecode/src/vm.rs` — VM interpreter
- `pata/cli/src/main.rs` — Use bytecode by default

**Complexity:** Large refactor, significant testing needed

---

**IIf Success Criteria:**
- [ ] All Phase II features still work under bytecode
- [ ] 2-3x performance improvement measured
- [ ] No correctness regressions

---

## Summary: All 30 Features

| # | Feature | Phase | Commits | Dependencies |
|----|---------|-------|---------|--------------|
| 1 | Drop Semantics | IIa | 2-3 | None |
| 2 | Exception Handling | IIa | 4-5 | Drop |
| 3 | Module Constants | IIa | 1-2 | None |
| 4 | Enums | IIa | 5-6 | Exception Handling |
| 5 | Pattern Matching | IIa | 4-5 | Enums |
| 6 | neno Module | IIa | 3-4 | None |
| 7 | ASI Stub Parsing | IIa | 1-2 | None |
| 8 | Generic Inference | IIb | 3-4 | IIa |
| 9 | Trait Bounds | IIb | 3-4 | Generic Inference |
| 10 | Type Aliases | IIb | 1 | IIa |
| 11 | Closures | IIb | 6-7 | Drop Semantics |
| 12 | Operator Overloading | IIb | 3-4 | IIa |
| 13 | String Interpolation | IIb | 2-3 | IIa |
| 14 | Format Strings | IIb | 1-2 | neno Module |
| 15 | Advanced File I/O | IIc | 4-5 | Exception Handling |
| 16 | JSON Support | IIc | 4-5 | IIa |
| 17 | Regex Module | IIc | 4-5 | IIa |
| 18 | HTTP Client | IIc | 3-4 | Exception Handling |
| 19 | Database Drivers | IIc | 3-4 | Exception Handling |
| 20 | Testing Framework | IId | 3-4 | IIa |
| 21 | Code Formatter | IId | 2-3 | IIa |
| 22 | Linter | IId | 2-3 | IIa |
| 23 | Doc Generator | IId | 2-3 | IIa |
| 24 | REPL | IIe | 4-5 | Evaluator |
| 25 | Package Manager | IIe | 4-5 | IIa |
| 26 | Debugger | IIe | 3-4 | Evaluator |
| 27 | Bytecode ISA | IIf | 15-20 | IIa+IIb |
| 28 | (Closures cont.) | - | - | - |
| 29 | (Operator Overload cont.) | - | - | - |
| 30 | (Pattern Match cont.) | - | - | - |

**Total Commits:** ~101-123  
**Total Timeline:** 6-8 months (aggressive)  
**Parallelization:** IIb+IIc run together, IId+IIe run together

---

## Starting Point: IIa Phase 1

**Week 1:** Drop Semantics + Module Constants  
**Week 2:** Exception Handling (foundational)  
**Week 3-4:** Enums (complex)  
**Week 5-6:** Pattern Matching + neno Module  
**Week 7:** ASI Stub Parsing + testing/documentation

Ready to begin IIa?
