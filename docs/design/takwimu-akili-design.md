# Takwimu/Safu (tensors/data) and Akili (NLP/LLM) design

**Status: not started, and not phased at all.** Unlike every other unimplemented subsystem in
this repo's design docs, Takwimu/Safu and Akili have **no entry anywhere in
[docs/spec/07-execution-and-roadmap.md](../spec/07-execution-and-roadmap.md)'s feature–phase
map** — confirmed by grep, neither "Takwimu," "Akili," "safu," nor "pakia" appears in that file
at all. Every other aspirational feature this session documented (borrow checker, data shapes,
concurrency, `wazi`/hardware, macros) has at least a phase assignment (III or IV) and a
dependency note. This doesn't. Nothing in the codebase references any of these names either —
confirmed by grep across `core/evaluator/src/builtins/*.rs` and `core/parser/src/*.rs`. This is
the earliest-stage of every subsystem covered in this batch of design docs.

## What the spec describes

Two subsections under `05-standard-library.md`'s "Moduli ya Takwimu na Safu (Data)":

- **Tensors and linear algebra**: `safu.umbo(dim)` (shape), `safu.zao_dot(a, b)` (dot product),
  `safu.geuza()` (transform/transpose), `safu.kawaida()` (normalize).
- **Transformation and calculus**: `safu.gradienti(kazi)` (gradient), `safu.takwimu(aina)`
  (statistics), `safu.badili(asili, lengo)` (convert between representations).
- **Data loading**: `pakia.csv(njia)`, `pakia.jozi(njia)` (load from JSON/pairs), `pakia.gawanya(data,
  uwiano)` (train/test split).

Separately, `06-tooling-and-ecosystem.md` describes **Akili** ("first-class primitives for
Swahili NLP and LLM interaction") as, for v1.1, "a submodule of Msingi (StdLib) under the
Takwimu/Safu surface" — i.e., the spec already positions Akili as living *inside* whatever
Takwimu/Safu becomes, not as a separate subsystem with its own module. It's also specified as
"conditionally compiled" (presumably via `#[sharti]`, though the spec doesn't say so explicitly)
"and can be excluded for minimal embedded targets" — a real constraint worth carrying into
whatever design eventually covers this: Akili/Takwimu needs to be a genuinely optional,
excludable compilation unit from day one, not bolted on as always-present later.

## Why this is a fundamentally different kind of "aspirational" than the others

Every other subsystem in this batch of design docs (borrow checker, data shapes, concurrency,
`wazi`) is a **language or runtime primitive** — something that extends what the interpreter
itself can express or execute, buildable with the tools already in this repo (Rust, the existing
`Value`/`ValueType` machinery, the existing evaluator). Takwimu/Safu is different: real tensor
operations, linear algebra, and gradient computation are themselves substantial pieces of
software (this is what libraries like `ndarray`, `nalgebra`, or bindings to BLAS/LAPACK exist to
provide in the Rust ecosystem) — implementing `safu.zao_dot`/`safu.gradienti` isn't "add a new
`Value` variant and a few evaluator match arms" the way `Seti<T>` is; it's "decide which
numerical-computing foundation to build or bind to, and design an entire tensor type and
operation set on top of it." Akili is further still: NLP/LLM primitives imply either shipping
model weights/inference code, or binding to an external inference runtime — neither of which
this interpreter's current architecture (a tree-walking evaluator over an AST, no FFI/dynamic
loading working yet per [wazi-hardware-design.md](wazi-hardware-design.md)) has any story for
yet.

## Open questions, before any implementation work is plausible

- **Should this even be built in-tree, or as an external package?** This repo already has a real
  (if narrow — path deps and locally-vendored version deps only, see
  [package-manager-design.md](package-manager-design.md)) package-dependency mechanism via
  `[tegemezi]`. A tensor/NLP library is exactly the kind of large, independently-versioned,
  optional-at-the-edges functionality that a package ecosystem is usually better suited to than
  a bundled stdlib module — especially given the spec's own "conditionally compiled... excluded
  for minimal embedded targets" framing already implies it should behave like an optional
  dependency, not a core language feature. This is worth deciding explicitly rather than
  defaulting to "it's stdlib because the spec put it under Msingi."
- **What does `safu`'s tensor type actually need at the `Value` level?** A dense, contiguous,
  typed numeric array is a fundamentally different shape from every existing `Value` variant —
  closer to `Bafa<T>` (the aligned-buffer type from
  [wazi-hardware-design.md](wazi-hardware-design.md)'s hardware surface) than to `Orodha<T>`
  (which is a `Vec<Value>` — boxed, heterogeneous-capable, not suited to numeric-array
  operations that need contiguous `f64`/`f32` storage for any real performance). Whether `safu`'s
  tensor type is built on the same `Bafa<T>` primitive `wazi` needs, or is its own independent
  `Value` variant, is a real design decision with a dependency on that doc's own open questions.
- **`pakia.csv`/`pakia.jozi` (file loading) interact with the file-I/O story already established
  for Wasm** (see [wasm-driver-design.md](wasm-driver-design.md)'s file-I/O section) — a browser
  target has no filesystem, so `pakia.*` functions need the same explicit-error-not-silent-no-op
  treatment `faili.rs`'s builtins already establish for `wasm32-unknown-unknown`, rather than
  inventing a new convention.
- **Does Akili need network access (for a hosted LLM API) or purely local inference?** These
  have completely different implementation shapes (an HTTP client + API-key management vs. an
  embedded model + inference engine) and the spec doesn't specify which. Given this
  interpreter's stdlib currently has no HTTP client anywhere (confirmed: no such builtin exists),
  network-based Akili would need that groundwork first — itself unscoped in any phase.

## Recommendation: defer scoping until phased

Given the total absence of phase assignment, the likely dependency on package-manager maturity
(registry/git fetching, currently explicitly out of scope per
[package-manager-design.md](package-manager-design.md), might be a prerequisite if this ends up
external), and the genuinely open "in-tree vs. external package" question above, this doc
intentionally stops short of proposing an order-of-work the way the other five docs in this batch
do. The first real step isn't implementation-shaped — it's deciding whether Takwimu/Safu/Akili
belongs in this repository's roadmap at all, or should be the first real test of the package
ecosystem once registry/git dependency fetching exists.

## Cross-references

- [implementation-status.md](implementation-status.md) — no existing checklist entry for this
  subsystem; it isn't in any phase list because it isn't in the roadmap's phase/feature map at
  all (confirmed above).
- [package-manager-design.md](package-manager-design.md) — the "should this be external" question
  depends on registry/git fetching maturity, which that doc documents as explicitly out of scope
  today.
- [wazi-hardware-design.md](wazi-hardware-design.md) — `Bafa<T>`'s potential relevance to a
  tensor `Value` representation.
- [wasm-driver-design.md](wasm-driver-design.md) — the file-I/O convention `pakia.*` should
  follow on Wasm targets.
- [docs/spec/05-standard-library.md](../spec/05-standard-library.md#moduli-ya-takwimu-na-safu-data)
  / [docs/spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md#akili--takwimu)
  — normative (but unphased) description this doc is drawn from.
