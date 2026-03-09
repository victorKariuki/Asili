# 0. Spec maintenance

[Overview](../SPECIFICATION.md) | Next: [Philosophy and EDP](01-philosophy-and-edp.md)

---

This document defines how the Asili specification is updated. Contributors and maintainers should follow this process so the spec remains a single source of truth.

---

## Spec versioning

Version and status live in [SPECIFICATION.md](../SPECIFICATION.md).

| Bump | When to use | Effect |
|------|--------------|--------|
| **Patch** (e.g. 1.0 → 1.0.1) | Wording, links, formatting only. No behavior change. | No new 08 entries; no new syntax/semantics. |
| **Minor** (e.g. 1.0 → 1.1) | New features, new keywords, new modules, new resolved decisions. Backward-compatible for existing Asili code. | Update 03–08 as needed; add 08 entries for new rules. |
| **Major** (e.g. 1.x → 2.0) | Breaking changes: removed or renamed keywords, changed semantics (e.g. `kama` used to panic, now returns `T?`), changed entry point or mandatory behavior, or a fundamental substrate shift (e.g. switching from GC-default to ownership-default memory). | Document migration; update all affected sections. |

---

## Where to edit

| Topic | Primary file(s) | Note |
|-------|------------------|------|
| Philosophy, EDP, pillars | [01-philosophy-and-edp.md](01-philosophy-and-edp.md) | |
| Project layout, file extensions, directories | [02-architecture-and-files.md](02-architecture-and-files.md) | |
| Keywords, entry point, imports, control flow | [03-syntax.md](03-syntax.md) | |
| Types, nullability, casting, references, lifetimes | [04-type-system.md](04-type-system.md) | |
| Stdlib modules, APIs, error model | [05-standard-library.md](05-standard-library.md) | |
| Pata, Mwalimu, tools | [06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md) | |
| Pipeline, memory, phases, workflow | [07-execution-and-roadmap.md](07-execution-and-roadmap.md) | |
| **Binding decisions** (semantics, defaults, “we do X not Y”) | [08-resolved-decisions.md](08-resolved-decisions.md) | **All rules go here**; other sections summarize or reference 08. |

**Rule:** If a change defines or changes a *rule* (e.g. “Orodha allocation fails like this”), add or update a subsection in 08. Keep 08 the only place that states the rule definitively.

---

## Process

1. **Propose** — Open an issue or patch. State what is changing, which files are affected, and whether the spec change is patch / minor / major.
2. **Locate** — Choose which section file(s) (01–08) and which 08 subsection (if a new rule).
3. **Edit** — Update content; update [SPECIFICATION.md](../SPECIFICATION.md) table of contents (and quick reference if needed); update [spec/README.md](README.md) section list if you add/remove/rename sections; fix **Previous | Next** links in affected spec files.
4. **Consistency check** — No contradiction with 08; no broken links; version/status in SPECIFICATION.md updated if this is a formal release.
5. **Review and merge** — Same as the contribution workflow: reviewer checks clarity and EDP alignment; subsystem maintainer (or spec owner) merges.

---

## Changelog and history

- **Changelog:** Maintain [spec/CHANGELOG.md](CHANGELOG.md) with short entries per spec version (e.g. `1.1 – Logic expansion, Chaguo, attributes`).
- **Tags:** When cutting a spec version, tag the repo (e.g. `spec-v1.0`, `spec-v1.1`).

---

## Checklist for any spec update

- [ ] Change is in the right section file(s) (01–08).
- [ ] Any new or changed **rule** is in 08 (and 08 is the only place that states the rule definitively).
- [ ] [SPECIFICATION.md](../SPECIFICATION.md) ToC and (if needed) quick reference updated.
- [ ] [spec/README.md](README.md) section list updated if sections were added, removed, or renamed.
- [ ] Previous/Next links correct in all touched spec files.
- [ ] No contradiction with 08 or between sections.
- [ ] Spec version and/or status updated in SPECIFICATION.md if this is a formal release.
- [ ] [spec/CHANGELOG.md](CHANGELOG.md) entry or tag created if recording a spec release.

---

Previous: [Overview](../SPECIFICATION.md) | Next: [Philosophy and EDP](01-philosophy-and-edp.md)
