# Implementation status

One place to check "is X actually done" before writing docs, examples, or new work. Supersedes
`phase-ii-decisions.md` (too narrow — Phase II is done and more has happened since). For the
language's *semantic* decisions (syntax, type system, ownership rules), see
[spec/08-resolved-decisions.md](../spec/08-resolved-decisions.md) — this doc is about
*implementation* status and tooling/architecture decisions, not language design.

Every claim below was checked directly against the code, not assumed from earlier docs — several
existing docs (this one's predecessor included) had drifted from reality. If you update behavior
covered here, update this doc in the same change.

---

## Phase checklist

Per [spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md)'s phase/feature map.

### Phase I — Catalyst (core interpreter)

- [x] Lexer, parser, semantic analysis, evaluator, diagnostics
- [x] `umbo`/`shughuli ya` (structs + methods — method's first param must be literally named
      `self`, typed with the struct: `kazi f(self: T, ...)`)
- [x] Orodha index `expr[index]`, Kamusi (`kamusi_tupu()`/`.ingiza`/`.pata`), Jozi (`jozi(a,b)`),
      Herufi casts, `linganisha` pattern matching on structs/Jozi/enums
- [x] `vunja`/`endelea`/`lebo` (break/continue/labeled loops)
- [x] `Seti<T>` — `HashSet<MapKey>`-backed set, `seti(...)`/`seti_tupu()`, always in scope via
      `msingi`. See [data-shapes-design.md](data-shapes-design.md).
- [x] `Namba_Kuu`/`Namba_Sahihi` — `num-bigint`/`bigdecimal`-backed. `namba_kuu_kutoka(neno)`/
      `namba_sahihi_kutoka(neno)` (via `leta hisabati`); infallible widening from `Namba`,
      fallible narrowing back (`Chaguo<Namba>`). Mixed arithmetic auto-widens; two `Namba_Kuu`
      operands stay in exact integer arithmetic rather than promoting to decimal. Needed a
      matching extension to the semantic analyzer's own (separate) binary-op and cast
      type-checking, not just the evaluator. See [data-shapes-design.md](data-shapes-design.md).
- [ ] `Mfululizo`, full borrow checker — Phase III, not started

### Phase II — Synthesis (LSP, Wasm, tooling)

- [x] `linganisha`, `jaribu`/`?`, `Sifa` (traits), `Jumla<T>` (enums as a real `Value` variant),
      `#[jaribio]` (test runner)
- [x] `#[sharti(lengo = "...")]` conditional compilation — predicate parser + pipeline filter
      pass; `pata.toml [jenga] lengo` / `pata jenga --lengo` select the build target. See
      [sharti-design.md](sharti-design.md) for the full architecture (grammar, scope limits,
      where the filter runs, and why `driver/wasm` doesn't run it).
- [x] Pakiti/Moduli — `pata-package` crate wired into `pata-cli` (real lockfile, SHA-256
      checksums); path dependencies and locally-vendored version dependencies resolve. Registry/git
      fetching is **not** implemented (no registry backend exists) — out of scope until one does.
      See [package-manager-design.md](package-manager-design.md) for the full architecture (the
      `pata.toml`/`Asili.toml` adapter approach, module layout, what's actually wired vs. dead code).
- [x] Wasm — `driver/wasm` builds for both `wasm32-unknown-unknown` (browser, `wasm-browser`
      feature, `console.log`/`console.error` via wasm-bindgen) and `wasm32-wasip1` (WASI,
      `wasm-wasi` feature — uses `std`'s native WASI support directly, no `wasi` crate needed).
      Multi-module `leta` resolves via `run_bundle` (in-memory bundler). Verified with real
      execution under Node's WASI loader and wasm-bindgen JS glue, not just compilation. See
      [wasm-driver-design.md](wasm-driver-design.md) for the full architecture (dual-target
      feature gating, the `platform.rs` I/O shim, the `run_bundle` bundler vs. the CLI's
      disk-based resolver, and known gaps like no cycle detection).
- [x] `Kasha_GC<T>` — opt-in (`leta kasha_gc`) reference-counted wrapper (`Rc<RefCell<Value>>`).
      Sharing via `.shirikisha()` (explicit, like Rust's `Rc::clone` — plain `weka b = a` still
      moves). Not a tracing/cycle-collecting GC — a self-referential `Kasha_GC<T>` leaks, by design.
      See [kasha-gc-design.md](kasha-gc-design.md) for the full architecture (method dispatch,
      opt-in wiring, `RefCell`-conflict handling, test coverage).
- [x] Mwalimu (LSP) — see [mwalimu-design.md](mwalimu-design.md); substantially more built than
      that doc used to claim (completion, goto-def, references, rename, workspace symbols all
      exist, not just diagnostics/hover).
- [x] `Faili`/`Mkondo`/`Kumbukumbu<T>` — resource handles, previously spec'd
      (`docs/spec/05-standard-library.md`) but entirely unimplemented. `Faili`/`Mkondo` close
      their OS handle on scope exit via a real `Drop` impl on the handle wrapper, not an
      `Env`-level hook (`Env::pop_scope` bypasses `Env::drop`, so a hook there would have missed
      ordinary scope exit). Not opt-in — available via the existing `leta faili`/`leta mfumo`
      (`Kumbukumbu<T>` via always-in-scope `msingi`), unlike `Kasha_GC<T>`. Satisfy
      `Inasomeka`/`Inandikika` by fiat, not via a checked `impl` — see the Sifa entry below for
      why. See [faili-mkondo-design.md](faili-mkondo-design.md) for the full architecture.
- [x] **Sifa (traits) — method-signature completeness.** `TraitDecl` now carries real method
      signatures (`sifa X { kazi f(self: Self) -> T }`, no bodies) instead of discarding its
      body at parse time; every `shughuli ya Target kwa/​: Trait` is checked against that list
      (`SEM105` on a missing/mismatched method). Deliberately no trait-object/dyn-dispatch — a
      trait only disambiguates method dispatch and gates completeness, not a value typed
      generically as "any Sifa." Found and fixed a real, previously-documented bug along the
      way: the `kwa`-keyword impl syntax had `target`/`trait_name` swapped, so a `kwa`-style
      trait impl's methods never dispatched at all (the `:`-colon syntax was always correct).
      `Inasomeka`/`Inandikika` are seeded built-in traits (like `Chaguo`/`Tokeo` via
      `standard_enums()`, not an `.asi` file — `.asi`'s loader is a line-by-line text parser
      that can't reliably parse a multi-line `sifa { }` body); `Faili`/`Mkondo` satisfy them
      *by fiat* (same method names/signatures), not via a real checked `impl`, since builtin
      `ValueType`s dispatch through hardcoded Rust match arms that never reach the
      `module.impls` lookup the completeness checker walks. See
      [sifa-traits-design.md](sifa-traits-design.md) for the full architecture.
- [ ] DAP (debugger) — not started; see [dap-later.md](dap-later.md).

### Phase III — Resolution (self-hosting, borrow checker) — decision made, implementation not started

Researched, decided, not built.

- [x] **Per-character string indexing** — `Neno.herufi_kwa(i) -> Chaguo<Herufi>`, grapheme-
      indexed (matching `.urefu()`'s counting). This was the one genuinely missing piece of
      string-manipulation coverage (`.urefu()`/`.biti_ngapi()`/`.kata()`/`.gawanya()`/
      `.badilisha()`/`.tafuta()` already existed — an earlier version of this entry wrongly
      claimed none of these existed).
- [x] **Lifetime-inference strategy decided**: region-based inference with a narrow, named-
      region (`muda M`) annotation surface, required only for the two cases
      `docs/spec/04-type-system.md` already names as needing explicit annotation (`Rejeo`/
      `Rejeo_Tenda` stored in `umbo` fields; function returns whose reference lifetime isn't
      inferable from a single parameter). This was framed as an open three-way choice, but the
      spec's own already-normative text ("lifetimes are inferred by default; explicit
      annotations are only required when references cross complex structural boundaries")
      ruled out the other two candidates directly — the decision was concretizing what the
      spec already committed to, not picking among genuinely open options. See
      [phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md)
      for the full syntax proposal and what it determines for implementation.
- [ ] **Runtime reference semantics — not implemented.** The semantic analyzer already tracks
      per-binding `moved`/`mut_borrowed`/`imm_borrows` (`core/parser/src/semantic/analyzer.rs`,
      `Binding` struct) with real diagnostics (SEM040–044) — a working compile-time move-checker.
      But the evaluator still clones everything regardless (`eval/expr.rs:217`'s
      `BorrowImm`/`BorrowMut` TODO) — move/borrow rules are enforced at compile time with no
      runtime backing yet. Unblocked by the decision above, not yet built.
- [ ] **`Mfululizo<T>`** — deliberately not given an `Rc`-based stopgap (considered and rejected;
      see the design doc) since it's the natural first user of the real-reference-semantics work
      once that lands. Still blocked, but on implementation now, not decision.
- [ ] Self-hosting itself (a lexer/parser/analyzer written in Asili) — not started, gated on the
      two items above.

### Phase IV — Nguvu (LLVM, embedded, concurrency, FFI) — partially started

- [x] **Concurrency (`tenda`/`njia`/`fungo`) — implemented**, despite the spec's own Phase IV
      placement (kept here rather than re-filed under Phase II, since the checklist above is
      organized by spec phase and this is the one exception). The naming discrepancy flagged in
      an earlier pass (`sambamba.rs`'s stub used `anza_mwendo`/`subiri_mwendo`, matching nothing
      in the spec) is resolved: functions renamed to `tenda`/`subiri_tenda` (module name
      `sambamba` kept). 1:1 OS-thread model (`std::thread`), not M:N green threads — a
      deliberate, recorded decision (`docs/spec/08-resolved-decisions.md`), not a stopgap. A
      spawned function's return value does not cross back through `subiri_tenda` (`Tokeo<Tupu,
      Neno>` only — success/panic); results flow through `njia`. `Kasha_GC<T>`/`Faili`/`Mkondo`
      rejected from crossing the thread boundary at all (checked recursively). Needed a new
      `SendValue` type (`core/evaluator/src/value/mod.rs`) — a structural, compiler-checked
      `Send`-safe mirror of `Value` — after discovering `unsafe impl Send` would have been
      required otherwise, and rejecting that route on soundness/maintenance grounds. `fungo`'s
      `.funga()`/`.fungua()` needed the `parking_lot`/`lock_api` crates for real manual
      lock/unlock (`std::sync::Mutex`'s guard can't span two separate calls). See
      [concurrency-design.md](concurrency-design.md) for the full architecture.
- [ ] **`syscall` (raw syscall stub, `core/evaluator/src/builtins/syscall.rs`) is callable from
      anywhere via `leta syscall` with no `wazi`-block gating**, even though the spec positions
      raw hardware/syscall-level access as `wazi`-only. A stub today (`FIXME(Phase IV)`, always
      returns `Anuani(0)`), so nothing is broken, but this needs a decision before real syscall
      dispatch is implemented. See [wazi-hardware-design.md](wazi-hardware-design.md).
- [ ] `kiungo` (FFI) is a documented stub: `core/evaluator/src/builtins/kiungo.rs`
      unconditionally returns `Err` from both exported functions, with a `TODO(Phase IV)`
      comment about `libloading`. Correctly scoped to this phase, not a surprise gap.
- [ ] `.asb` is not real bytecode — `core/evaluator/src/asb.rs` bincode-serializes the parsed
      AST `Module`; running an `.asb` file re-interprets the AST via the tree-walk evaluator. A
      separate, genuinely-started-but-incomplete bytecode VM (`core/evaluator/src/bytecode.rs`,
      `core/evaluator/src/tir.rs`) exists with a real TODO trail (it names exactly which opcodes
      are missing), but nothing in `pata jenga`'s default pipeline ever emits `format=bytecode`
      — it's disconnected from the path anyone actually uses.

---

## What's NOT on the phase roadmap but matters before "production ready"

Found by directly auditing the code this cycle, not from the spec:

- **No CI pipeline exists at all.** No `.github/workflows/`, confirmed directly. Every test
  suite, every lint pass, every example verification only runs when someone runs it by hand.
  This is the single highest-leverage gap — everything else here is only as trustworthy as the
  fact that someone remembers to check it.
- **Semantic-analyzer test coverage is thin: 1 of 51 `SEM0xx` diagnostic codes has a test that
  deliberately triggers and asserts it.** `semantic_check_with_env_and_modules` (the
  cross-module-aware entry point `pata-cli` actually uses) has zero direct test callers. This is
  exactly the class of gap that let three real bugs ship silently this cycle (see below).
- **`pata-lint`'s rules are thinner than they look.** LINT201 (repeated-string-literal detection)
  used to just count *all* string literals in a file and call anything over 5 "repeated" — fixed
  this cycle, but the other rules (LINT001-003 naming, LINT101 length, LINT202 docs, LINT203
  unused imports) haven't had the same scrutiny.
- **`pata thibitisha` checks 2 of the 6 things its own TODO comment says it should** (doc coverage
  + formatting only; missing type-stability, ABI-compatibility, trait-completeness, and a test-
  coverage threshold check).
- **`pata nadhifu` (formatter) is a line-level text transform, not an AST-based formatter** — its
  own source comments admit it can corrupt string literals containing `{`/`}`/`,` via blind
  brace/comma replacement.

## Real bugs found and fixed this cycle (worth knowing about, not re-introducing)

All found by accident while writing/fixing example programs and docs — none were caught by any
existing test, which is the throughline for the test-coverage gap above.

1. **`check_type_compatibility`** used to push a build-failing `SEM-INF` diagnostic even when it
   had already decided something was "compatible" — contradicting its own logic. Fixed: no longer
   pushes a diagnostic on the permissive path.
2. **Module-level constants couldn't be referenced from within their own file.** `use_ident()`
   checked imported constants and local scope, but never the current module's own top-level
   `thabiti` declarations. Fixed: added `local_constants` lookup.
3. **The actual root cause of #2 (and worse than #2 alone suggested): `thabiti X: Type = value`
   silently discarded its type annotation.** `parse_module_constant()` never consumed the `:`
   before the type, so `parse_type()` swallowed it into a bogus string the type parser couldn't
   recognize, always falling back to `Unknown` — meaning **every explicitly-typed module constant
   in the codebase had its declared type silently discarded** until this was fixed.
4. **`runtime` module's semantic export table was missing 5 functions that were already
   implemented** (`arch`, `ni_debug`, `ni_wasm`, `mazingira`, `muda_wa_kuanza`) — `leta runtime`
   could never actually use them; every call was rejected as "unknown function."
5. **Struct fields separated by newlines instead of commas** (`umbo P { x: Namba\n y: Namba }` —
   a real, used pattern, see `examples/phase1_modules`) only "worked" by accident, because a
   *different* bug (`parse_type()` swallowing tokens past a line it shouldn't) happened to paper
   over the struct-field parser never actually looping for a second field. Fixing the swallowing
   bug exposed the struct-field bug; both are now fixed.
6. **`thabiti`/module-level `tupa` (drop) semantics** — already fixed *before* this cycle (an
   earlier doc, `PHASE_II_IMPL.md`, described this as an open bug; it wasn't anymore, and that
   stale doc has been removed).

7. **Decimal float literals appeared not to parse** (`3.6` alone read as `3` `.` `6`, a field
   access, producing `SEM099`/a runtime field-access error) — first observed while verifying
   `docs/repl/hisabati.md`'s examples. **Re-investigated and no longer reproduces**: `weka x =
   3.6` and `x kama Neno` now both work correctly in the REPL and in compiled programs
   (verified directly). The lexer's decimal-literal logic (`core/lexer/src/lib.rs`, the
   digit-dot-digit check) was already correct; the actual symptom was very likely the same
   `parse_type()`-swallows-adjacent-tokens bug class as #3/#5 above, in a form not caught by
   that earlier fix — specifically item #12 below (`kama X kama Y` chaining swallowing the
   second cast's type into the first). Leaving this entry as a historical note in case it
   resurfaces, but do not re-add doc caveats about decimal literals not working.

Also fixed this cycle, found while verifying every code example under `docs/language/` and
`docs/repl/` against the real compiler (a much larger sweep than the type-checker bugs above):

8. **Division by zero panicked instead of producing IEEE-754 infinity/NaN**, contradicting the
   spec's own documented `Ukomo`/`-Ukomo`/`Siyo_Namba` semantics (`spec/04-type-system.md`).
   Fixed: `/` no longer special-cases `b == 0.0`; raw `f64` division naturally yields
   `Ukomo`/`-Ukomo`/`Siyo_Namba`. The `kama Neno` cast previously rendered these via Rust's raw
   `f64` `Display` (`inf`/`-inf`/`NaN`); now renders the documented Swahili names.
9. **Global constants beyond `Ukomo`/`Siyo_Namba` were seeded into the runtime but rejected by
   the semantic analyzer.** `PI`, `E`, `PHI`, `TOLEO`, `JINA_OS`, `SEKUNDE_KWA_SIKU`, and 13
   others (`env.rs`'s `seed_global_constants()`) all evaluate fine at runtime but were unknown
   to `use_ident()` in the analyzer, so any program referencing them failed to compile
   (`SEM045: jina halijulikani`) despite `docs/language/11-maadili-ya-kimataifa.md` claiming
   they're always in scope. Fixed: `use_ident()`'s hardcoded exception list now matches the
   runtime's full seed list.
10. **Struct and pair (`Jozi`) destructuring patterns didn't bind their variables at the
    semantic-analysis layer**, even though the evaluator's pattern matcher already bound them
    correctly at runtime — `linganisha p { Pika { x: a, y: b } => { ...a...b... } }` failed to
    compile (`SEM045: jina halijulikani`) for `a`/`b`, and the identical bug affected `Jozi`
    pattern destructuring `(n, neno) => ...`. Fixed: added binding logic in `check_stmt`'s
    `Stmt::Match` handling, mirroring the existing `Pattern::Enum` binding arm (struct fields
    bind with their declared field type; Jozi elements bind as `Unknown`).
11. **List index-assignment (`a[i] = val`) was unimplemented for `Orodha`.** `a[i] = val` always
    lowers to a call to `.ingiza(i, val)` (shared with `Kamusi`'s `m[k] = v`), but only the
    `Kamusi` arm existed — using it on a list threw a runtime type error. Fixed: added an
    `Orodha` arm (bounds-checked, in-place element replacement) plus the matching analyzer type
    contract.
12. **`kama Type1 kama Type2` (chained casts) silently produced garbage when written inline as
    a call argument** — `chapisha(65 kama Herufi kama Neno)` compiled and ran with **no
    output and no error**. Root cause: `parse_type()` didn't stop at the `kama` keyword, so
    parsing the first cast's type greedily swallowed `Herufi kama Neno` as one bogus type name;
    the second `kama` was consumed as part of the (invalid, `Unknown`-typed) first type, and the
    outer cast's evaluator fallback (`else { Ok(v) }` for an unrecognized type string) silently
    passed the original `Namba` value through unchanged into a `Neno`-typed argument slot with
    no type-check catching it. Fixed: `parse_type()` now also stops at `kama`, matching the
    stop-list it already used for `,`/`)`/`{`/`}`/`=`/`->`. The same construct split across two
    statements (`weka h = 65 kama Herufi; weka s = h kama Neno`) always worked correctly — only
    the single-expression chained form was affected.
13. **`kweli`/`si_kweli kama Namba` always produced `0`, regardless of the boolean's value.**
    `value::as_f64` (the shared numeric-coercion helper used by the `Namba` cast) has no
    `Value::Ukweli` arm, so the whole coercion chain fell through to `unwrap_or(0.0)` even for
    `kweli`. Fixed: added an explicit `Ukweli` arm at the cast site (not in the shared `as_f64`
    helper, to avoid changing behavior for arithmetic/comparison call sites that also use it) —
    `kweli kama Namba` now correctly gives `1`, `si_kweli` gives `0`.
14. **`Tokeo.ni_sawa()` (the `Ok`-check counterpart to the already-working `.ni_kosa()`) was
    unimplemented for the real runtime `Value::Tokeo` representation** — only wired for a
    legacy `Value::Enum(en, ...)`-shaped "Tokeo" that nothing in the current evaluator actually
    produces. Calling `.ni_sawa()` on any real `Tokeo` value threw `aina: mwito wa njia
    'ni_sawa' unahitaji Neno, Orodha, jenum au umbo`. Fixed: added the missing
    `(Value::Tokeo(res), "ni_sawa")` arm (the semantic analyzer's type contract already allowed
    it — only the evaluator was missing the case).
15. **`Tokeo::Sawa(v)`/`Tokeo::Kosa(e)`/`Chaguo::Kuna(v)`/`Chaguo::Hamna` patterns in `linganisha`
    silently never matched a builtin-produced `Tokeo`/`Chaguo` — no error, the arm just never
    ran.** Two separate runtime shapes exist for these: `Value::Tokeo`/`Value::Chaguo` (from
    `gawio`, `kamusi.pata`, casts, etc.) and `Value::Enum("Tokeo"/"Chaguo", ...)` (only from
    explicit `Tokeo::Sawa(x)`-style construction, since the parser registers real built-in
    `jenum` declarations for them). `match_and_bind_pattern`'s `Pattern::Enum` arm only ever
    compared against `Value::Enum`, so matching the constructor-style pattern against a
    builtin-produced value fell straight through to `_`/the next arm with no diagnostic at all.
    Fixed: `match_and_bind_pattern` now special-cases `enum_name == "Tokeo"`/`"Chaguo"` and
    matches directly against `Value::Tokeo`/`Value::Chaguo`'s real shape (binding the wrapped
    value/error) before falling back to the `Value::Enum` path. This was the single largest
    correctness gap found this cycle — every doc example that worked around it by re-calling a
    Tokeo-producing function fresh inside a wildcard-only `linganisha` (rather than matching
    `Ok`/`Err` directly) can now be simplified; `docs/language/05-makosa.md` was rewritten
    accordingly.
16. **`+` between a `Neno` and an unresolved generic-placeholder type (`TypeVar`, e.g. a
    `Tokeo::Kosa(e)` pattern's bound `e`) was rejected even when the value is genuinely a
    string at runtime.** `Expr::Binary`'s `Add` arm checked `l == ValueType::Neno` by strict
    equality instead of using the analyzer's own `compatible()` helper (which already treats
    `TypeVar`/`Unknown` as wildcards everywhere else) — so `"Kosa: " + e` failed to compile with
    `SEM033` for any `e` whose static type came from an unresolved enum generic parameter, even
    though passing `e` alone to a `Neno`-typed parameter (e.g. `chapisha(e)`) worked fine. Fixed:
    `Add`'s Neno/Namba checks now also accept a `TypeVar`/`Unknown` operand paired with a
    concretely-typed one (never both-wildcard, to avoid silently guessing with no information).
    Directly enabled by fixing #15 above — `Tokeo::Kosa(e) => ...` bindings are now common enough
    to make this a real, not just theoretical, gap.
17. **`Kamusi.idadi()` (entry count) type-checked but crashed at runtime.** The semantic
    analyzer's method-type table already listed `(Kamusi, "idadi") -> Namba`, but no matching
    evaluator dispatch arm existed — calling it threw `aina: mwito wa njia 'idadi' unahitaji
    Neno, Orodha, jenum au umbo` despite compiling cleanly. Fixed: added `(Value::Kamusi(m),
    "idadi") => Namba(m.len())`.
18. **`Kamusi.funguo()`'s return type was untyped, breaking any chained call on it** (e.g.
    `m.funguo().urefu()` failed `SEM039`, `aina 'Unknown' haina njia`), **and `weka_key` (the
    documented alias for `ingiza`) had no analyzer type contract at all**, even though both were
    already wired at the evaluator-dispatch level. Fixed: added
    `(Kamusi(k,_), "funguo") -> Orodha(k)` and folded `"weka_key"` into the existing `"ingiza"`
    arm in the analyzer's method-type table.
19. **`umbiza()`'s calendar-date formatting used fixed 30-day months and no leap-year
    handling** (`core/evaluator/src/builtins/majira.rs`) — dates could be off by more than two
    weeks depending on time of year (the time-of-day portion was always correct). Fixed:
    replaced the ad-hoc arithmetic with Howard Hinnant's `civil_from_days` algorithm (pure
    integer math, no new dependency), which correctly handles the proleptic Gregorian calendar
    including leap years.
20. **`Biti16`/`uBiti16` casts never range-checked — any value "fit," unlike every other
    fixed-width integer type.** The cast-to-integer range-check match (`core/evaluator/src/
    eval/expr.rs`) had explicit arms for `Biti8`/`Biti32`/`Biti64`/`uBiti8`/`uBiti32`/`uBiti64`
    but fell through to `_ => true` for the 16-bit variants. Fixed: added the missing
    `Biti16`/`uBiti16` range checks (`i16`/`u16` bounds).
21. **`hisabati`'s domain-checked functions (`mzizi`, `faktoriali`, `baki`, `logi*`, `asini`,
    `akosini`, `akosini_h`, `atanjenti_h`) returned a `Struct("Kosa_Hisabati", ...)` error value,
    inconsistent with every other fallible `hisabati` function (`gawio`, `mizizi`, `kipeo`,
    `upeo`), which return a plain `Neno` error string** — meaning `.kosa()`/a matched `Err(e)`
    gave a debug-dumped struct for half of `hisabati`'s fallible functions and a clean string
    for the other half, entirely undocumented either way. Fixed: `kosa_h` (the shared helper
    behind the struct-shaped errors) now returns a plain `Value::Neno`, matching the rest of the
    module; the corresponding `FnContract`s (`namba_namba_tokeo_namba`/`namba_tokeo_namba`) now
    declare `Tokeo<T, Neno>` instead of `Tokeo<T, Unknown>`.
22. **`Tokeo`/`Chaguo`'s built-in variant names were English (`Ok`/`Err`/`Some`) despite the
    spec already resolving Swahili names for them** (`docs/spec/08-resolved-decisions.md`:
    `SAWA`/`KOSA`; `docs/spec/04-type-system.md`: `Kuna(T)`) — a real spec-vs-implementation
    divergence, not just a doc gap. Renamed throughout: `core/parser/src/parse.rs`'s
    `standard_enums()` now registers `Tokeo::Sawa`/`Tokeo::Kosa` and `Chaguo::Kuna`/
    `Chaguo::Hamna` (title-case, matching `Hamna`'s existing casing convention — the spec's
    ALL-CAPS `SAWA`/`KOSA` reads as prose emphasis elsewhere in the same document, not a literal
    casing rule); every runtime match on the old variant-name strings in
    `core/evaluator/src/eval/expr.rs` updated to match. Also gave `Value` a manual `Debug` impl
    (`core/evaluator/src/value/mod.rs`) instead of `#[derive(Debug)]`, since `Tokeo`/`Chaguo`
    wrap Rust's own `Result`/`Option` — the derived impl would otherwise print the Rust-side
    `Ok(..)`/`Err(..)`/`Some(..)` literally in raw/REPL output regardless of the Asili-level
    rename. Every other `Value` variant's Debug output is unchanged (same shape as the derive
    produced). `Hamna` (already Swahili) was untouched.

Known, deliberately-not-fixed gaps in the type checker (small, contained, not yet done):

- The builtin-method-call fallback (`analyzer.rs`, method dispatch on Neno/Orodha/Kamusi/etc.)
  silently returns `Unknown` for any unrecognized method name, instead of raising the same
  "unknown method" diagnostic that struct/enum methods already get. A typo'd method call on a
  builtin type is currently invisible to the checker.
- Generic constructors (`kamusi()`, `kosa()`, `kasha_gc_unda()`) don't propagate their argument's
  real type the way `orodha`/`jozi`/`chaguo` already do — same fix shape, just missing arms.
- **Trait-impl method dispatch (`shughuli ya X kwa Trait`) doesn't wire the method into method
  lookup at all** — calling it (`x.method()`) fails with `SEM040: njia haipo`, even though the
  identical body under a plain `shughuli ya X { ... }` (no `kwa Trait`) dispatches correctly.
  Not a "static dispatch in progress" nuance — trait impls are simply invisible to method
  resolution today. Bigger than a one-arm fix (needs impl-block-to-trait association wired into
  whatever resolves `x.method()`); found while verifying `docs/language/07-mfumo-wa-aina.md`.
- **A `Tokeo` binding is only marked "consumed" by `linganisha` or `?`/`jaribu`, not by calling
  a method like `.ni_kosa()`/`.kosa()` on it.** `weka r = f(); ikiwa r.ni_kosa() { ... }` fails
  `SEM048` even though `.ni_kosa()` is a legitimate, documented way to inspect a `Tokeo`. Since
  `Tokeo::Sawa(v)`/`Tokeo::Kosa(e)` patterns now match correctly against builtin-produced `Tokeo`
  values (fix #15 above), the direct-`linganisha` path no longer needs this workaround — only
  the method-call-based check (`.ni_kosa()`/`.ni_sawa()` on a stored variable) still hits
  `SEM048`. `docs/language/10-mifano.md`'s method-based example still needs the re-call-fresh
  workaround; its `linganisha`-based examples don't. Fixing this means also marking consumption
  on method calls in `mark_tokeo_consumed`'s call sites, which needs care not to allow
  genuinely-unchecked uses through.
- **`kama Neno` doesn't unwrap `Chaguo<Neno>`/`Tokeo<Neno,_>`** — casting one falls through to
  the catch-all `format!("{v:?}")` branch, producing a debug-format string
  (`Chaguo(Kuna(Neno("x")))`) instead of unwrapping to `"x"`. Whether a cast should auto-unwrap
  is a real design question (what does `Hamna kama Neno` mean?), not an obvious bug fix — use
  `.angu(default)` to unwrap explicitly instead.
- **`#[sharti(...)]` only recognizes the `target` key** — `#[sharti(os = "linux")]` or any other
  key is rejected outright (`SHA001`), not silently ignored. If OS-level (as opposed to
  wasm/native target) conditional compilation is wanted, it doesn't exist yet. See
  [sharti-design.md](sharti-design.md) for the full list of known gaps (no statement-level
  gating, no negation, `driver/wasm` doesn't run the filter).
- **`a.kila_mmoja(f)` requires the callback name as a string literal** (`a.kila_mmoja("f")`),
  not a bare function reference (`a.kila_mmoja(f)` fails `SEM045: jina halijulikani`) — there
  are no first-class function values yet, just this string-based dispatch convention.
- **Binary (`0b...`) and hex (`0x...`) numeric literals are rejected outright** (`PAR072`/
  parser), decimal-only by explicit design choice — not a bug, but worth knowing before writing
  any example that wants bit-pattern-literal syntax.

---

## Recommended order of work (not yet started, prioritized)

1. **Fix the decimal-literal lexer bug** (#7 above) — this is a correctness bug affecting any
   program that uses a float literal directly, not just a doc-polish item.
2. **Stand up CI** — `cargo test --workspace`, `pata-lint` across `examples/`, `pata jenga` on
   every example project, on every push/PR. Everything else is unverifiable without this.
3. **Semantic-analyzer test sweep** — target the ~50 untested `SEM0xx` codes. Natural occasion to
   split `core/parser/src/semantic/analyzer.rs` (~1650 lines) and `core/parser/src/parse.rs`
   (~1380 lines, one `impl` with every `parse_*` method) by concern, since that's exactly where
   all the bugs above were hiding, untested.
4. **The two small analyzer fixes** listed above.
5. **Defer Phase III** until per-character string indexing is added and a lifetime-inference
   strategy is actually decided — starting the borrow checker before that is building on a
   decision that hasn't been made yet, which the roadmap itself already warns against. See
   [phase3-self-hosting-borrow-checker-design.md](phase3-self-hosting-borrow-checker-design.md).
6. Lower priority, not blocking: DAP, a one-command install script, a broader example set (most
   of the 11 current examples are single-feature test fixtures rather than realistic programs —
   `examples/astar` is the one exception).

Design docs exist for every aspirational (not-yet-implemented) subsystem the spec describes, for
whenever work on them starts — not implementation guides, but a record of what's already
half-there, what's genuinely blocked and on what, and the open decisions each needs before
starting:
[data-shapes-design.md](data-shapes-design.md) (`Mfululizo`/`Seti`/`Namba_Kuu`/`Namba_Sahihi`),
[concurrency-async-design.md](concurrency-async-design.md) (`tenda`/`njia`/`fungo`/`sawia`/
`subiri`, plus the `sambamba` naming discrepancy),
[wazi-hardware-design.md](wazi-hardware-design.md) (unsafe blocks, hardware primitives, plus the
ungated `syscall` finding), [takwimu-akili-design.md](takwimu-akili-design.md) (tensors/data
loading/NLP — unphased in the roadmap, unlike everything else here), and
[kielelezo-macros-design.md](kielelezo-macros-design.md).
