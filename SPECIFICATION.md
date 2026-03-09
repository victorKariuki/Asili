# Asili v1.1 — Master Specification

**Version:** 1.1  
**Status:** Draft

Asili (Origin/Nature) is a programming language that bridges high-level human intuition and low-level machine precision. Swahili is the primary language of the logic; the toolchain (Pata) supports learning, applications, and embedded systems through a single, transparent pipeline. This document is the entry point to the formal specification.

---

## Table of contents

0. [Spec maintenance](spec/00-maintenance.md) — How to update the spec
1. [Philosophy and EDP](spec/01-philosophy-and-edp.md) — Core pillars, Environmental Diagnostic Protocol, architectural philosophy
2. [Architecture and Files](spec/02-architecture-and-files.md) — Project structure, file extensions, scalable layout
3. [Syntax](spec/03-syntax.md) — Swahili keywords (Signal), entry point, imports, loops, bitwise/shift
4. [Type System](spec/04-type-system.md) — Numeric tiers, primitives, reference types, nullability, casting
5. [Standard Library](spec/05-standard-library.md) — Msingi, Mfumo, Data/Takwimu, Hardware (wazi), error model
6. [Tooling and Ecosystem](spec/06-tooling-and-ecosystem.md) — Pata, Mwalimu (LSP), Akili/Takwimu
7. [Execution and Roadmap](spec/07-execution-and-roadmap.md) — Pipeline, memory, phases, contribution workflow
8. [Resolved Decisions](spec/08-resolved-decisions.md) — Definitive resolutions for conflicts and open questions

---

## Quick reference

| Category   | Swahili              | Technical        |
|-----------|----------------------|------------------|
| Variable  | `weka` / `thabiti`   | Mutable / Immutable |
| Logic     | `kazi` / `rejesha`   | Function / Return |
| Structure | `umbo` / `shughuli ya` | Struct / Impl (Traits) |
| Flow      | `ikiwa`, `au_ikiwa`, `vinginevyo` | If, Else If, Else |
| Loops     | `wakati`, `kwa`      | While, For       |
| Control   | `linganisha`, `vunja`, `endelea`, `lebo` | Match, Break, Continue, Label |
| Errors    | `jaribu`, `?`        | Try / propagate  |
| Panic     | `paparika`           | Unrecoverable abort (panic) |
| Ownership | `azima`, `azima_tenda`, `tupa` | Borrow immutable/mutable, explicit drop |
| Numeric edges | `Ukomo`, `Siyo_Namba` | Infinity / NaN for `Namba` |
| Entry     | `kazi kuu(hoja: Orodha<Neno>) -> Tupu` | Main |

Key types: `Namba`, `Neno`, `Ukweli`, `Orodha<T>`, `Kamusi<K,V>`, `Tokeo<T,E>`, `Chaguo<T>` (or `T?`), `Tupu`, `Hamna`, `Sifa`, `Jumla<T>`.
