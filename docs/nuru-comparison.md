# Nuru vs Asili: What We Learn (Comparison)

Reference: Nuru project layout and patterns. This document summarizes what Asili misses or does the hard way, and has been updated to reflect Asili’s actual layout (lib/std, builtins, tegemezi, lib/docs).

---

## 1. **Debugging (DAP)**

- **Nuru:** `cmd/nuru-dap`, docs in `docs/tooling/LSP_AND_DAP.md`.
- **Asili:** No Debug Adapter Protocol; no breakpoints, stepping, or variable view. DAP is documented as planned for a later phase; see [docs/design/dap-later.md](design/dap-later.md).
- **Takeaway:** Adding a DAP server later is a separate, sizeable chunk of work; Nuru has already made that investment.

---

## 2. **LSP structure**

- **Nuru:** LSP split into focused files: `completion.go`, `diagnostics.go`, `references.go`, `symbols/table.go`, `position.go`, `protocol.go`, `server.go`.
- **Asili:** One `pata/lsp/src/lib.rs` with diagnostics, hover, and document store.
- **Takeaway:** Asili is doing it the “one big module” way. Nuru’s split makes it easier to add completion, goto-def, references, or document symbols one by one and test them in isolation.

---

## 3. **Editor integration (VSCode extension)**

- **Nuru:** Full VSCode extension: syntax (`nuru.tmLanguage.json`), snippets, language config, LSP client.
- **Asili:** No extension; users must configure `pata mwalimu` or `pata-lsp` themselves.
- **Takeaway:** Asili is doing it the hard way for “open and it works”: no one-click install, no syntax highlighting unless the editor uses the LSP.

---

## 4. **Evaluator / parser file layout**

- **Nuru:** One file per construct — e.g. evaluator: `assign.go`, `block.go`, `call.go`, `for.go`, `if.go`, `switch.go`, …; parser: `arrays.go`, `for.go`, `function.go`, …
- **Asili:** One large `eval.rs` (~650 lines), one `parse.rs` with all `parse_*` in a single `impl`.
- **Takeaway:** Asili’s single-file style is simpler at first but will get harder as you add constructs (merge conflicts, “where does this live?”). Nuru’s “one construct per file” scales better and keeps changes localized.

---

## 5. **Stdlib / builtins layout**

- **Nuru:** `module/` with one file per domain: `crypto.go`, `fs.go`, `http.go`, `json.go`, `mfumo.go`, `os.go`, `path.go`, `regex.go`, `time.go`, `url.go`.
- **Asili:** **Interface:** `lib/std/` with one `.asi`/`.as` per module (hisabati, mfumo, moduli). **Implementation:** one `builtins.rs` (evaluator) plus `builtin_modules.rs` (CLI contracts). Builtin modules can be resolved without any `.asi` file (Phase I).
- **Takeaway:** The stdlib *surface* is already one-module-per-file in `lib/std`. The single-file growth is in Rust: splitting `builtins.rs` by module (hisabati, mfumo, neno, …) would mirror Nuru’s layout and scale better.

---

## 6. **REPL and inline docs**

- **Nuru:** `repl/` with `repl.go` and per-topic docs in `repl/docs/` (en + sw): arrays, files, hisabati, keywords, etc.
- **Asili:** Spec says “terminal REPL” in Phase I but there’s no REPL; only “copy example + `pata jenga --tenda`”.
- **Takeaway:** Asili is missing the REPL entirely. Nuru also ties REPL docs to the runtime (e.g. `?hisabati`), which Asili could do later.

---

## 7. **Structured docs (howto, design, audits)**

- **Nuru:** `docs/howto/` (e.g. 00-getting-started, UNITS_SUPPORTED_BY_CODEBASE), `docs/solutions/tooling/nuru-lsp-design.md`, `docs/tooling/LSP_AND_DAP.md`, and `upgrade_nuru_road_to_bigLeague/` with audit docs.
- **Asili:** `spec/` + README; `lib/docs/` for `.asdoc` templates (spec: generate docs via `pata maelezo`). Less “how do I do X?” and “how was this designed?” at repo root.
- **Takeaway:** Asili could add a short howto (e.g. “run a program”, “run tests”, “use LSP”) and design/audit notes (e.g. “Mwalimu design”, “Phase II decisions”) in `docs/` without changing code. `lib/docs` remains the place for stdlib API doc templates.

---

## 8. **Examples breadth**

- **Nuru:** Many examples: crypto, fs, http server, json, path/njia, regex, sudoku, perceptron, stress tests, etc.
- **Asili:** Five examples: hello, hisabati, umbo_na_shughuli, linganisha_orodha, README.
- **Takeaway:** Asili is doing less “show the language and stdlib in action.” Adding a few more (e.g. file I/O, Tokeo/jaribu, Kamusi, a small algorithm) would make onboarding and demos easier.

---

## 9. **Analysis as its own layer**

- **Nuru:** `analysis/` (e.g. `analysis.go`, `datastructure_analyzer.go`, `module_analyzer.go`) separate from parser.
- **Asili:** Semantic analysis lives inside the parser crate (`semantic/analyzer.rs`, `types.rs`).
- **Takeaway:** Nuru can run “analysis” without always doing a full parse; useful for tooling and LSP. Asili could later extract a small `asili_analysis` (or similar) for LSP/IDE without tying it to the full pipeline.

---

## 10. **Runtime “object” layout**

- **Nuru:** `object/` with one file per kind: `array.go`, `dict.go`, `file.go`, `function.go`, `regex.go`, …
- **Asili:** Single `value.rs` with one big `Value` enum.
- **Takeaway:** Asili’s enum is simpler and fits Rust. As each variant gets more behavior, splitting by “kind” (e.g. value/orodha.rs, value/kamusi.rs) could avoid one huge file, similar to Nuru’s object split.

---

## 11. **Format as its own package**

- **Nuru:** `format/format.go` at repo root.
- **Asili:** Formatting inside `pata/cli` (`pipeline/format.rs`).
- **Takeaway:** Minor; Nuru makes “format” a clear unit. Asili could expose a thin `pata-nadhifu` or `asili_format` API if other tools (e.g. LSP format-on-save) need it.

---

## 12. **Install and build UX**

- **Nuru:** `sh/install.sh`, `Makefile` for common tasks.
- **Asili:** Only `cargo`; no one-command install or make targets.
- **Takeaway:** Asili is “the hard way” for non-Rust users. A small install script and a Makefile (or a single “how to install” in the README) would lower friction.

---

## 13. **Stdlib written in the language / third-party modules**

- **Nuru:** `third_party/math` with `hesabu.nr`, `test.nr` (stdlib in Nuru).
- **Asili:** **Built-in stdlib:** Implemented in Rust (`builtins.rs`, `builtin_modules.rs`); `leta hisabati` / `leta mfumo` work without `.asi` files. **Stdlib surface in tree:** `lib/std/` has `.asi`/`.as` (hisabati, mfumo, moduli) — interface-first placeholders; same modules as builtins. **Third-party:** `[tegemezi]` in `pata.toml`; dependencies (Swahili-named, semver), managed by `pata ongeza`.
- **Takeaway:** Asili has both: (1) built-in stdlib (Rust impl, no disk required), (2) stdlib surface in the language in `lib/std`, and (3) third-party modules via tegemezi. “Stdlib in the language” is already present as `lib/std`; the implementation for core modules is currently Rust. A Nuru-style “third_party” or contrib set of modules written in Asili could be added later.

---
