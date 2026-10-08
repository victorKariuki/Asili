# Performance bottlenecks — measured, ranked, with fixes

A sweep of every engine and phase, October 2026: what is slow, why (from profiles, not guesses),
what fixes it, and what has been done. Numbers are best-of-N wall time on a 4-core x86-64 VM
(±5% noise) or exact instruction counts from callgrind where small changes need to be visible.

## Workloads

The examples in `examples/` all finish in ~2 ms — process start-up — so they cannot show
bottlenecks. Seven small programs each stress one thing (kept in the session scratchpad; each is
a few lines and easy to recreate from the descriptions):

| Workload | What it does | Asili native | Tree-walker | Reference |
|---|---|---|---|---|
| desimali | 20 M iterations of float multiply-add | 146 ms | 6.9 s | C −O2 146 ms |
| ungo | sieve of 5 M | 24 ms | 5.4 s | C −O2 56 ms |
| orodha | push 1 M numbers, sum with `kwa x katika` | 15–17 ms | 435 ms | C −O2 12 ms |
| fib | recursive `fib(30)`, 1.35 M calls | 14 ms | 796 ms | C −O2 5.9 ms |
| umbo | build a 2-field `umbo` 1 M times through a `kazi` | 3 ms (was 150–185 ms) | 590 ms | C −O2 1.9 ms |
| kamusi | 300 k `Kamusi` updates with `Neno` keys | ~70 ms (was 75–90) | 184 ms | Python 66 ms |
| maneno | 300 k numbers to text + join; 20 k `s = s + "ab"` | 60 ms (was 71–79) | 105 ms | Python 64 ms |

Sudoku (`examples/sudoku/bench/run.sh`): asili-nguvu 5.2 ms, clang −O2 5.9 ms, 90,665 attempts.

Summary: numeric code compiled by nguvu is at or above C. Everything that touches non-numeric
values (structs, maps, strings, methods) leaves native code for the host on every operation and
is 1–80× off its reference. Calls between numeric functions cost ~2.4× C.

## Fixed in this round

| # | Bottleneck | Evidence | Fix | Result |
|---|---|---|---|---|
| F1 | `Value` was 56 bytes | every clone, drop and move copies it | struct/field/enum/variant names interned (`asili_parser::Name`, 8 bytes); `Kamusi`, `Seti`, `Namba_Kuu`, `Namba_Sahihi` behind `Rc` | 32 bytes; tree-walker Sudoku 11.53 → 9.95 G instructions |
| F2 | Copying a `Kamusi`/`Seti` copied every entry | tree-walker map loop 6.0 s | copy on write (`Rc::make_mut`), like `Orodha` | 6,034 → 184 ms (33×) |
| F3 | A `Vec` allocated and freed per builtin/method call in the host | 900 k allocations in the map loop | one reused argument buffer | −3% map loop |
| F4 | Trace spans cost ~37 instructions per host call with tracing off | `asili_trace::open` in profiles | inline off-check, traced path `#[cold]` | −2% struct loop |
| F5 | Builtin table built twice at every start | 249 k of 1.08 M start-up instructions | built once, sorted once | 1.08 → 0.98 M |
| F6 | Small functions called through the host; structs built on the heap per iteration | ~600 instructions per call, ~280 per struct built, ~100 per field read | inlining of `rejesha`-only functions when compiling bytecode (`inlinable`, guarded by `CheckDepth`), then scalar replacement of structs that never leave their function (`scalars.rs`) | struct loop 1,805 M → 9 M instructions, ~150–185 → 3 ms (C 1.9 ms) |
| F8 | `s = s + t` copied the whole string every time | 45% of the strings workload in `memcpy` | `Neno` storage is `Text` (header + bytes, spare capacity), appended in place when unshared (`ops::assign_in_place`, both engines) | 830 → 427 M instructions, 71 → 60 ms (Python 64) |
| F9 | Boxing numbers, copying values and loading constants each went through the host's general instruction path | ~100 instructions of dispatch per instruction | direct runtime calls from native code (`BoxNum`, `BoxBool`, `ValMov`, `ConstVal`) | map loop −9%, strings −6% |
| F10 | Every list push called the runtime | ~85 instructions per push | inline append when there is room, the length stored into the list; runtime only to grow | list workload 119.6 → 37.8 M instructions |
| F7 | x86-64 prologues pushed every callee-saved register | 10 push/pop per call in small functions | push only the registers the function uses (AArch64 already did) | no change on fib (it uses all five); smaller frames elsewhere |

Earlier the same day: token kinds and interned names in the parser and tree-walker, parallel and
reproducible native builds, incremental LSP parsing (see `CHANGELOG.md`).

## Open, ranked by impact

### 1. Direct calls between numeric functions (fib: 2.4× C)

*Evidence*: 133 instructions per call in generated code (C: ~15). Per call the caller writes the
arguments to a memory buffer, the callee spills its four pointer arguments to the stack and
reloads them, saves all five callee-saved registers whatever it uses, reads parameters back from
memory, writes the result to memory, and the caller checks a status word and loads the result.

*Fix*: a register calling convention for direct entries — `f64` arguments in `xmm0–7`/`v0–7`,
the result in `xmm0`/`v0`, the status in `rax`/`x0`; write the buffer only on the cold host-call
path (depth or stack limit); save only the callee-saved registers the function uses. Touches
`lower.rs`, `codegen.rs`, `codegen_a64.rs`, `regalloc.rs`; bumps `ABI_VERSION`. Expected ~2× on
call-heavy numeric code.

### 2. Every non-numeric operation is a round trip to the host (umbo 78×, kamusi, maneno)

*Evidence*: 100–200 instructions per operation before any work: native code spills registers,
calls `native_exec`, which looks the instruction up again, tries `numeric_op`, then matches the
whole `Opcode` enum in `exec_slow`; then native code reloads. A struct loop iteration is ~1,800
instructions; the map loop ~3,000.

*Fix*: typed runtime entry points that native code calls directly for the hot non-numeric
operations — field read by slot, struct build, map get/set with a `Neno` key, string concat,
list push — with their operands as arguments, no generic decode, and no spill of registers the
operation does not read. Then dedicated opcodes for the hot methods (`pata`, `angu`, `ingiza`,
`ongeza`, `urefu`, `idadi`) chosen when the bytecode is compiled.

### 3. Calls to functions that take or return values go through the host

*Evidence*: ~600 instructions per call: `frame_for` (pooled frame, resized and cleared),
`invoke` (depth, stack check), `run_native`, status decoding.

*Done for small functions* (F6): a function whose body is one `rejesha` of a pure expression is
inlined. *Still open* for larger ones: direct entries for functions with value registers (pass a
frame pointer, as the host does), and inlining of bodies with statements, method calls or `?`.

### 4. Struct layout

*Partly done* (F6): a struct that never leaves its function is no longer built at all. Structs
that are stored, passed or returned still are, and for those:

*Evidence*: every struct instance carries its own list of `(field name, value)` pairs; building
one allocates and fills that list, and a field read by name searches it.

*Fix*: `Struct(Rc<StructType>, Rc<[Value]>)` — the field names live once in the type, a read is
an index (already known at compile time: `FieldNum`'s `slot`), and native code can read a field
without the host once the layout is fixed.

### 5. Method dispatch by string

*Evidence*: ~250 instructions per method call to decide it is pure (`is_pure_method`: a linear
search of names) and then find it (`pure_method`: a `match` over (receiver, name string)).

*Fix*: resolve the method to an id once — when compiling bytecode (`MethodOp` carries it) and,
for the tree-walker, when parsing (interned `Name` → id table) — and `match` on the id.

### 6. Building a string by appending is quadratic — fixed (F8)

*Evidence*: `s = s + "ab"` 20,000 times copies 400 MB (45% of the strings workload); `Neno` is an
immutable `Rc<str>`, so every append copies the whole string. CPython appends in place when the
string is not shared.

*Fix*: a `Neno` with spare capacity that appends in place when its owner is unique (the same
copy-on-write rule as lists and maps).

### 7. Tree-walker (200× native)

Matters wherever there is no native backend (wasm, other CPUs). *Evidence*: Sudoku 1.0 s vs
5.2 ms; expression dispatch, variable lookup by scanning scopes, value clone/drop.

*Fix*: resolve each local to a slot index when the function is analysed (no scan), and compile
expressions to closures instead of matching the syntax tree at every step. Typically 2–4×.

### 8. Native build: range analysis is 59% of it

*Evidence*: `analyze_numbers` joins dense per-register state vectors at every block edge.
*Fix*: track only the registers live at each block, or run the analysis on SSA values.

### 9. Start-up (~1 ms above a bare process)

*Evidence*: 35% dynamic relocation of the PIE runner, 15% building ~350 boxed builtin closures
keyed by `String`. *Fix*: a static, non-PIE runner; a static builtin table (`fn` pointers,
`&'static str` names).

### 10. LSP requests still parse the whole document

Hover, inlay hints, semantic tokens and symbols each lex and parse the full text per request.
*Fix*: take the tree from `DocStore::parse` (already incremental for diagnostics).

### 11. Silent fallback to the tree-walker

A `kazi` the bytecode compiler cannot lower runs ~200× slower, silently, in debug builds (release
builds refuse, naming the `kazi`). *Fix*: a warning from `pata jenga` naming each such `kazi`.

## Accuracy finding — fixed

The semantic analyzer accepted a call to a method that does not exist (`a.panga()` on an
`Orodha<Namba>` compiled, then failed at run time with a misleading message). Fixed in
`0e60649` (issue #76): the method lists live once in `asili_parser::builtins`, the analyzer
reports `SEM040`, and a test checks the engines implement every listed method.

## Next

In order (list pushes are done, F10): the direct-call
convention (fib); method ids instead of name matching; local slots and closure compilation for
the tree-walker.
