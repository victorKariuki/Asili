---
name: manage-project-boards
description: Keep the Asili GitHub Projects (project 16 "Asili bug tracker", project 17 "Asili Feature release", both under owner victorKariuki) in sync with real work — move an item's Status as work on it starts/ships, and file+add new items for gaps discovered along the way. Use whenever starting, finishing, or discovering work that corresponds to (or should become) an item on either board — not only when the user explicitly asks to "update the board" or "check the project."
---

# Keeping the Asili GitHub Project boards current

This repo has two GitHub Projects (v2, user-owned, not repo-owned — `gh project ... --owner
victorKariuki`), both scaffolded with the same Status pipeline options (bug tracker additionally
has `To triage` before `Backlog`): `To triage` → `Backlog` → `Ready` → `In progress` → `In review`
→ `Done`.

- **Project 16 — "Asili bug tracker"**: known bugs, gaps, deferred edge cases — things confirmed
  not to work or explicitly out of scope for the current pass.
- **Project 17 — "Asili Feature release"**: planned/in-progress features, scoped from
  `docs/design/pata-production-readiness.md` and `implementation-status.md`.

A board that doesn't reflect real state is worse than no board — don't let items sit at `Backlog`
after work has actually started, or stay open after the fix has shipped.

## Workflow

1. **Before starting substantive work**, check whether it corresponds to an existing item on
   either board (`gh project item-list 16 --owner victorKariuki` /
   `gh project item-list 17 --owner victorKariuki`, or `gh issue list --repo victorKariuki/Asili`
   for open issues not yet on a board). Match by subject, not by exact title wording.
2. **When starting work on a tracked item**, move its Status to `In progress` (see commands
   below). Do this at the start of the work, not retroactively at the end — the board should
   reflect what's happening now, not just what's finished.
3. **When work is fully shipped and merged** (committed, tests passing, pushed per this repo's
   usual push-confirmation rules — see `release-and-git-flow`), move the Status to `Done` and
   close the underlying issue (`gh issue close <n> --repo victorKariuki/Asili`) if one exists.
   Don't mark `Done` before the work has actually landed on a shared branch.
4. **When you discover a real, scoped gap** while working (a deferred edge case, a documented
   "not implemented" comment, a genuine bug) that isn't already tracked, create a GitHub issue
   (`gh issue create --repo victorKariuki/Asili`) with concrete detail — cite the actual file/line
   or design-doc section, not a vague restatement — then add it to the appropriate board
   (`gh project item-add <16|17> --owner victorKariuki --url <issue-url>`) with a Priority
   (`P0`/`P1`/`P2`) and Status reflecting its real state (usually `To triage`/`Backlog`, not
   `Ready` — see the exception below). Don't leave real findings undocumented just because the
   current task didn't ask for a board update.
5. **An item that's already fixed but needs follow-up verification** (e.g. a regression test not
   yet written for something already corrected) belongs at `Ready`, not `To triage`/`Backlog` —
   match the item's actual state, not a default.

## Commands (gh CLI, v2 Projects API)

Get a project's numeric field IDs and option IDs once per session (they're stable but not
memorized — don't guess them):

```bash
gh project field-list 16 --owner victorKariuki --format json
gh project field-list 17 --owner victorKariuki --format json
```

Add an existing issue to a board:

```bash
gh project item-add <16|17> --owner victorKariuki --url https://github.com/victorKariuki/Asili/issues/<n>
```

Set Status/Priority on an item (needs the project's own numeric ID — `PVT_...` — the item's own ID
— `PVTI_...` — and the field's option ID; `gh project item-edit` does not take human-readable
option names):

```bash
gh project item-edit --project-id <PVT_...> --id <PVTI_...> --field-id <PVTSSF_...> --single-select-option-id <option-id>
```

To read current Status/Priority across a board's items in one call (faster than paging through
`item-list`'s default text output):

```bash
gh api graphql -f query='
query {
  node(id: "<PVT_...>") {
    ... on ProjectV2 {
      items(first: 20) {
        nodes {
          content { ... on Issue { number title } }
          status: fieldValueByName(name: "Status") { ... on ProjectV2ItemFieldSingleSelectValue { name } }
          priority: fieldValueByName(name: "Priority") { ... on ProjectV2ItemFieldSingleSelectValue { name } }
        }
      }
    }
  }
}'
```

## Auth note

Writing to a project (descriptions, adding items, editing fields) needs the `project` OAuth
scope; reading needs `read:project`. Both require an interactive device-code flow
(`gh auth refresh -s project --hostname github.com`) that can't be driven non-interactively — if a
`gh project` write command fails with a missing-scope error, tell the user to run that command
themselves and wait for confirmation before retrying, rather than attempting a workaround.

## What NOT to do

- Don't create an issue for something already fully tracked elsewhere (check first).
- Don't mark an item `Done` speculatively ("this should work now") — only after verifying
  (tests passing, build clean) per this repo's normal verification bar.
- Don't bulk-reorganize the boards' Status pipeline or field options without being asked — the
  current 5-6-stage setup is GitHub's sensible default and already fits this repo's needs.
