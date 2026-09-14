# `#[sharti(...)]` conditional compilation design

`#[sharti(...)]` gates a top-level item so it's only compiled for certain build targets — the
Asili equivalent of Rust's `#[cfg(...)]`, but far narrower in scope (see below). The
implementation is split across two crates: `core/parser` (predicate parsing/evaluation, no
target-selection state) and `pata/cli` (the pipeline pass that actually filters a `Module`, plus
target selection).

## Predicate grammar

`core/parser/src/attrs.rs` — `parse_sharti_predicate(args: &str) -> Result<ShartiPredicate, String>`.

Grammar: `key = "value" ('|' "value")*`. As implemented:

- **Only one key is recognized: `lengo`.** Any other key (`#[sharti(os = "linux")]`,
  `#[sharti(debug = "true")]`, etc.) is rejected outright with `sharti key isiyojulikana: "<key>"
  (pekee "lengo" inatambulika)` — not silently ignored, a hard error.
- **Values are OR'd with `|`**: `#[sharti(lengo = "wasm" | "native")]` survives for either
  target. Each value must be a quoted string (`lengo = wasm` without quotes is a parse error);
  an empty quoted value (`lengo = ""`) is also rejected.
- **No `!`/`&`, no nesting.** There's no negation and no AND-within-one-predicate — only the
  OR-list above. AND is achieved a different way (see below), not via predicate syntax.

`ShartiPredicate` is currently a one-variant enum (`Target(Vec<String>)`) — the grammar has room
to grow (more keys, and/or combinators) without a breaking change to the attribute syntax itself,
but nothing beyond `target = "a" | "b"` parses today.

## Evaluation: `item_survives`

`core/parser/src/attrs.rs::item_survives(attrs: &[Attribute], target: &Target) -> Result<bool, String>`:

- An item with **no** `#[sharti(...)]` attribute always survives (returns `Ok(true)`) —
  `#[sharti]` is opt-in gating, not a required annotation.
- **Multiple `#[sharti(...)]` attributes on the same item are ANDed together**: every predicate
  must allow the active target, or the item is dropped. This is how AND-of-conditions is
  expressed, since the predicate grammar itself has no `&`: stack two `#[sharti(lengo = "a")]`
  attributes rather than writing one compound predicate.
- A malformed predicate (bad key, missing `=`, unquoted/empty value) returns `Err(String)` rather
  than `Ok(false)` — a parse problem is distinct from "doesn't match this target," and is
  surfaced as a diagnostic rather than causing the item to silently vanish (see below).

## Scope: whole top-level items only

Exactly five AST node types carry an `attrs: Vec<Attribute>` field (confirmed by grep across
`core/parser/src/ast.rs`): `EnumDecl`, `StructDecl`, `TraitDecl`, `ImplDecl`, `Function`. Nothing
else — `Stmt`, `Block`, `Param`, and constants have no `attrs` field. This means `#[sharti(...)]`
can only gate a whole function/struct/trait/impl/enum, never a single statement, a branch inside a
function body, or a parameter. There is no way to write "this one line only runs on wasm" inside
an otherwise-shared function — the function itself has to be split, or gated as a whole.

## The pipeline filter pass

`pata/cli/src/pipeline/sharti.rs::filter_module_for_target(module: &mut Module, target: &Target)
-> Result<(), Vec<Diagnostic>>`:

- `retain`s each of `module.enums`/`structs`/`traits`/`impls`/`functions` by calling
  `item_survives` on that item's `attrs`.
- On a malformed predicate (`item_survives` returns `Err`), the item is **kept** (not dropped)
  and a `SHA001` diagnostic (`"sharti isiyoeleweka: <message>"`, stage `"sharti"`, spanned at the
  item's line) is collected. Keeping the item is deliberate — per the source comment, "so a
  single bad predicate doesn't silently vanish code; the collected diagnostic will fail the
  build" — a broken predicate is a compile error, not a silent no-op or a silent keep-anyway.
- If any errors were collected, the whole pass returns `Err(Vec<Diagnostic>)`; the pipeline
  caller wraps this into a build failure via `diag_err("sharti", errors)`.

### Where it runs in `pata/cli/src/pipeline/compile.rs`

The filter runs **after parsing, before semantic analysis** — filtered-out items never need to
type-check for a target that doesn't build them. It runs three times per compile, against three
different `Module`s in sequence:

1. Right after parsing the entrypoint file (before dependency resolution).
2. Against each resolved dependency's module (`program.resolved.values_mut()`), after
   `resolve_all`.
3. Against `program.merged_for_eval` — the fully merged module — before it's handed to semantic
   analysis / evaluation.

This three-pass structure repeats identically across `compile_project`, `compile_single_file`,
and the two other compile-path variants in `compile.rs` (four call sites total, twelve
`filter_module_for_target` calls) — every compilation path filters the entrypoint, every resolved
dependency, and the merged result, in that order, before anything downstream sees the module.

## Target selection

`resolve_target(cli_target: Option<&str>, manifest_target: Option<&str>) -> Target`
(`compile.rs`): `cli_target.or(manifest_target).unwrap_or("native")`. Precedence, highest first:

1. **CLI flag** — `pata jenga --lengo <lengo>`.
2. **Manifest key** — `pata.toml`'s `[jenga] lengo = "..."` (`pipeline/project.rs`).
3. **Default** — `"native"` if neither is set.

There is no validation that a given target string means anything beyond being a string compared
against `#[sharti(lengo = "...")]` predicates — `pata jenga --lengo banana` compiles
successfully and simply matches nothing gated to `"native"`/`"wasm"`/etc. `Target` is a bare
`pub struct Target(pub String)` newtype (`core/parser/src/attrs.rs`); there's no enum of "known"
targets anywhere.

Build caches (`.asb-cache/`, the project input-hash) are target-aware — switching `--lengo`
between builds forces a rebuild rather than serving a stale artifact built for a different
target (confirmed via `cache_key`/`project_input_hash`'s target parameter in `compile.rs`).

## Not run everywhere: `driver/wasm`

`filter_module_for_target` is a `pata/cli` pipeline pass — `driver/wasm`'s `run_source`/
`run_bundle` entry points call straight into lex/parse/evaluate with no `#[sharti]` awareness at
all (see [wasm-driver-design.md](wasm-driver-design.md#not-yet-implemented--known-gaps)). Source
handed directly to the Wasm driver (bypassing `pata jenga --lengo wasm`) still contains any
`#[sharti(lengo = "native")]`-gated items; they simply aren't filtered out. In practice this only
matters if such an item is actually reachable from `kuu` and does something target-inappropriate
— the item type-checks and evaluates fine either way, since the evaluator itself has no
`#[sharti]` concept; filtering is purely a `pata/cli`-pipeline-time decision.

## Tests

`core/parser/src/attrs.rs` unit tests: single-value and OR-value predicate parsing, unknown-key
rejection, missing-`=`/unquoted-value rejection, no-`#[sharti]`-always-survives, single-predicate
gating, and multi-attribute ANDing.

`pata/cli/src/pipeline/sharti.rs` unit tests:
- `gated_function_filtered_by_target` — a `#[sharti(lengo = "wasm")]`-gated function is present
  pre-filter, absent after filtering for `"native"`, present again after filtering (a fresh copy)
  for `"wasm"`; an ungated `kuu` survives both.
- `gated_struct_filtered_by_target` — same pattern for a gated `umbo`.
- `unrecognized_predicate_key_is_an_error` — `#[sharti(bahati = "x")]` makes
  `filter_module_for_target` return `Err`.

End-to-end (`pata/cli/src` integration tests, referenced from
[implementation-status.md](implementation-status.md)): a fixture project built with `[jenga]
lengo = "wasm"` containing a `#[sharti(lengo = "native")]`-gated function does not appear in the
emitted `.asb` when deserialized — confirming the filter actually changes what gets compiled, not
just what the in-memory `Module` looks like mid-pipeline.

## Known gaps

- **Single recognized key (`lengo`).** No OS-level, debug/release, or feature-flag style
  conditions exist — `#[sharti(os = "linux")]` is a hard error, not a supported (or even
  silently-ignored) predicate.
- **No statement/body-level gating.** A function that's 90% shared and 10% target-specific has
  no way to gate just the target-specific part — the whole function has to be duplicated or
  restructured to call out to two separately-gated helper functions.
- **No `!`/AND-in-one-predicate.** AND requires stacking multiple `#[sharti(...)]` attributes;
  there's no negation at all (no way to say "everything except wasm" without listing every other
  target explicitly).
- **`driver/wasm` doesn't run the filter** — see above.
- **No validation of target names** — typos in `--lengo`/`lengo` compile silently and just
  match nothing.

## Cross-references

- [implementation-status.md](implementation-status.md#phase-ii--synthesis-lsp-wasm-tooling) —
  phase checklist entry this doc supersedes with detail.
- [wasm-driver-design.md](wasm-driver-design.md) — the `wasm`/`native` targets `#[sharti]` was
  built to distinguish, and the one driver that doesn't run this filter.
- [docs/spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) /
  [docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md) — normative
  mentions of conditional compilation and the Phase II feature map.
- [mwalimu-design.md](mwalimu-design.md) / [package-manager-design.md](package-manager-design.md)
  — sibling design docs this one follows in structure/style.
