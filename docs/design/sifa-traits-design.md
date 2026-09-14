# Sifa (traits): method-signature completeness design

`Sifa` went from nominal-only (a `TraitDecl` carrying just a name — its body was discarded by
`skip_body()` at parse time) to carrying real method signatures, checked for completeness
against every `shughuli ya X kwa/​: Trait` block that names it. Scope, deliberately: **method
signatures and a completeness check only — no trait-object/dyn-dispatch.** There is still no
`Value` variant for "a value that implements trait X" and no generic-over-trait function
parameters. That is real, separate future work, not implied by anything below.

## A real bug found and fixed on the way in

Before any of this could be tested, `shughuli ya Paka kwa Inayoonyeshwa` (the `kwa`-keyword impl
syntax) turned out to be completely broken — `core/parser/src/parse.rs`'s `parse_impl_decl` had
`target`/`trait_name` swapped for this syntax specifically (the colon syntax, `shughuli ya
Target: Trait { }`, was always correct). `imp.target` ended up holding the *trait* name instead
of the struct name, so nothing — not method dispatch, not (now) completeness checking — ever
matched a `kwa`-style impl against its actual struct. This was previously documented in
`docs/language/07-mfumo-wa-aina.md` as a "known bug," reproduced and confirmed, then fixed as
part of this work (it directly blocked testing the completeness checker with the `kwa` syntax).
See `core/evaluator/tests/traits.rs`'s `kwa_syntax_trait_impl_method_dispatches` /
`colon_syntax_trait_impl_method_dispatches` / `plain_impl_without_trait_still_works` for the
regression coverage.

## The `TraitDecl`/`TraitMethodSig` shape

`core/parser/src/ast.rs`:

```rust
pub struct TraitDecl {
    pub name: String,
    pub methods: Vec<TraitMethodSig>,   // new
    pub line: usize,
    pub column: usize,
    pub attrs: Vec<Attribute>,
    pub is_public: bool,
}

pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<Param>,      // reuses Function's Param type
    pub return_type: TypeExpr,   // reuses Function's TypeExpr
    pub line: usize,
}
```

`Self` inside a trait method's params/return type is a literal type-name string (`"Self"`),
resolved to the implementing type's real name only at completeness-check time — the trait
declaration itself doesn't know what will implement it.

## Parsing

`core/parser/src/parse.rs`'s `parse_trait_decl` no longer calls `skip_body()`. It parses zero or
more bodiless `kazi name(params) -> ReturnType` signatures inside `{ }` (no `{ }` at all is a
valid empty/marker trait with no requirements — `empty_trait_body_has_no_completeness_requirements`
covers this). Reuses the existing `parse_params()`/`parse_type()` helpers `parse_function`
already uses, rather than duplicating param/return-type parsing logic. An omitted `-> ReturnType`
defaults to `Tupu`, matching how a function with no explicit return type would be written
elsewhere in the language.

## The completeness check

`core/parser/src/semantic/analyzer.rs`'s `check_trait_completeness`, called once per
`ImplDecl` inside the existing `for i in &self.module.impls` loop (the same loop that already
validates every impl method's `self` parameter, SEM100/SEM101):

- Skips silently if the impl doesn't name a trait, or names one this module doesn't know about
  (an unresolved/typo'd `trait_name` is a separate SEM007-class concern, not this check's job).
- For every method the trait requires, resolves `Self` in its params/return type to the impl's
  own `target` type name, then looks for a matching method in the impl's `body` by name, full
  parameter-type list (order-sensitive, count-checked), and return type — not just name/arity.
- Missing or mismatched methods produce `SEM105: njia '<method>' ya sifa '<trait>'
  haijatekelezwa kwa '<target>'` ("method X of trait Y is not implemented for Z"), one
  diagnostic per missing/mismatched method, at the `impl` block's own line.

`Param`/`TypeExpr` already derive `PartialEq`, so the signature comparison is a straightforward
structural `==` after `Self`-resolution — no new comparison machinery needed.

## `Inasomeka`/`Inandikika`: seeded, not `.asi`-defined

The natural-looking choice — define these in an `.asi` prelude file under `lib/std/`, matching
the spec's "also exposed as an in-language surface" framing — turned out to be unsafe to take:
`.asi` files are parsed by a hand-rolled line-by-line text parser
(`pata/cli/src/pipeline/interface_registry.rs`), explicitly documented (pre-existing TODO) as
unable to reliably parse anything spanning multiple lines, which a `sifa { ... }` body
inherently does.

Instead, `core/parser/src/parse.rs` gained `standard_traits()`, seeded into every parsed
`Module` the same way `standard_enums()` already seeds `Chaguo`/`Tokeo`/`Kuna`/`Hamna`/`Sawa`/
`Kosa` — a hardcoded Rust literal, not text parsed from anywhere, so it can't be misparsed:

```rust
pub(crate) fn standard_traits(&self) -> Vec<TraitDecl> {
    vec![
        TraitDecl { name: "Inasomeka".into(), methods: vec![/* soma(self) -> Tokeo<Neno, Neno> */], .. },
        TraitDecl { name: "Inandikika".into(), methods: vec![/* andika(self, data: Neno) -> Tokeo<Tupu, Neno> */], .. },
    ]
}
```

## Why `Faili`/`Mkondo` satisfy these traits *by fiat*, not via a checked `impl`

This is a real, deliberate limitation, not an oversight — worth being explicit about since the
original plan for this phase assumed a real `impl Inasomeka for Faili { ... }` block would work.
It structurally can't, today:

- `Faili`/`Mkondo` are builtin `ValueType` variants (`core/parser/src/ast.rs`). Their methods
  (`.soma()`/`.andika()`/`.funga()`) are hardcoded Rust match arms in
  `core/evaluator/src/eval/expr.rs` and `core/parser/src/semantic/analyzer.rs`'s
  `is_builtin`-gated dispatch — the *same* mechanism `Kasha_GC<T>`/`Tokeo`/`Chaguo` methods use.
- `is_builtin` short-circuits **before** the analyzer ever reaches the `module.impls` lookup
  that would consult a real `shughuli ya Faili kwa Inasomeka { ... }` block (confirmed directly
  in `analyzer.rs`: the `is_builtin` branch returns its own hardcoded return-type match and
  never falls through to the `imp.target ==` search).
- The new completeness checker only walks `self.module.impls` — real, user-parsed impl blocks.
  There is no `ImplDecl` for `Faili`/`Inasomeka` anywhere to check, because Asili source can't
  target a builtin `ValueType` with `shughuli ya` at all (`target` is matched against
  user-declared struct/enum names, not builtin type names).

So: `Faili`/`Mkondo` are documented as implementing `Inasomeka`/`Inandikika` (same method names,
same signatures — `.soma() -> Tokeo<Neno, Neno>`, `.andika(data) -> Tokeo<Tupu, Neno>`), and
that conformance is true by construction (the signatures match), but it is asserted, not parsed
or checked by the compiler. If real generic-over-trait dispatch is ever built (the
trait-object/`dyn`-`Value` work explicitly out of this phase's scope), reconciling builtin types
with real trait-object conformance is the point where this would need revisiting — most likely
by having the analyzer's `is_builtin` branch consult a small hardcoded
"which traits does this builtin type satisfy" table rather than only its own method-dispatch
match arms, so a hypothetical `param: Inasomeka` could accept a `Faili` value. Not attempted
here.

## Tests

`core/evaluator/tests/traits.rs`:

- `kwa_syntax_trait_impl_method_dispatches` / `colon_syntax_trait_impl_method_dispatches` /
  `plain_impl_without_trait_still_works` — the `kwa`-swap bug fix, both impl syntaxes, and the
  no-trait case all dispatch correctly.
- `incomplete_impl_is_rejected_sem105` — a trait requiring two methods, an impl providing only
  one, correctly fails with `SEM105` naming the missing method.
- `impl_with_mismatched_return_type_is_rejected_sem105` — same method name/arity but a wrong
  return type does not satisfy the trait.
- `empty_trait_body_has_no_completeness_requirements` — a `sifa Marker { }` with no methods
  imposes no requirements on its implementers.

## Cross-references

- [faili-mkondo-design.md](faili-mkondo-design.md) — `Faili`/`Mkondo`'s `Inasomeka`/
  `Inandikika` "by fiat" conformance, and the method-dispatch mechanism this doc explains is
  incompatible with real trait-object checking.
- [docs/spec/04-type-system.md](../spec/04-type-system.md) — normative Sifa entry.
- [docs/spec/05-standard-library.md](../spec/05-standard-library.md#resource-handles-and-traits)
  — `Inasomeka`/`Inandikika`'s stdlib entry.
- [docs/language/07-mfumo-wa-aina.md](../language/07-mfumo-wa-aina.md) — tutorial coverage,
  including the now-fixed `kwa`-syntax bug note.
- [kasha-gc-design.md](kasha-gc-design.md) — the sibling design doc this one's structure follows.
