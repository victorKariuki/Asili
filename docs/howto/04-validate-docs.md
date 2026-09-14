# Validating docs and format

`pata thibitisha` checks that a project meets basic publishing hygiene before a release/PR.

## Basic usage

```bash
pata thibitisha
```

On success:

```
thibitisha: sawa
```

Exit 0. On failure, exits 1 with a message naming the first problem found.

## What it checks today

1. **The project compiles** (`pata jenga` runs first).
2. **Public documentation coverage** — every `umma kazi` (public function), `umma umbo`
   (struct), and `umma sifa` (trait) needs a `///` doc comment on the line directly above it:

   ```asili
   /// Adds two numbers.
   umma kazi jumla(a: Namba, b: Namba) -> Namba {
     rejesha a + b
   }
   ```

   Missing one fails with `nyaraka zimekosekana kwa kazi ya umma '<name>' kwenye <file>:<line>`
   (or the `umbo`/`sifa` equivalent).
3. **Trait completeness** — every `sifa` reachable from the project (declared locally, or by a
   `leta`-ed dependency module/`.asi` stub) must have at least one `shughuli ya X kwa/​: Trait`
   impl somewhere in the project. This is a project-wide check, distinct from `SEM105` (which
   only fires when an impl already names a trait but gets its signatures wrong) — a trait with
   **zero** impls anywhere previously compiled and shipped silently. Failing case:

   ```asili
   sifa Inayoonyeshwa { kazi onyesha(self: Self) -> Neno }
   // no `shughuli ya ... kwa Inayoonyeshwa` anywhere in the project
   ```

   Fails with `sifa '<name>' haina utekelezaji wowote kwenye mradi huu`. Built-in seeded traits
   (`Inasomeka`/`Inandikika`) are exempt — they're satisfied by fiat for builtin types
   (`Faili`/`Mkondo`), not something a project itself implements.
4. **FFI-safety of `#[kiunganishi]` signatures** — every parameter and return type of a
   `#[kiunganishi]`-tagged function must be a primitive safe to cross a C ABI boundary (`Namba`,
   `Ukweli`, `Herufi`, `Tupu`, `Anuani`, or a fixed-width `Biti*`/`uBiti*` type) — not
   `Orodha`/`Kamusi`/a struct/any other heap-owning or generic-container type. `kiungo`/FFI
   itself is still a Phase IV stub with no C-signature declaration syntax yet (see
   [implementation-status.md](../design/implementation-status.md)), so this isn't full ABI
   compatibility checking (there's no declared C signature to compare against yet) — it's the
   real, checkable prerequisite: types that could never be ABI-safe regardless of what that
   future contract looks like. Failing case:

   ```asili
   #[kiunganishi]
   kazi kutoka_c(x: Orodha<Namba>) -> Namba { ... }
   ```

   Fails with `kazi ya kiunganishi '<name>' hoja '<param>' ina aina isiyo salama kwa ABI ya C:
   <type>`.
5. **Formatting** — the project must already be in canonical `pata nadhifu` form; run `pata
   nadhifu` first if you see `mafaili hayajafuata muundo sahihi: tumia \`pata nadhifu\` kwanza`.
6. **Test coverage ratio** (opt-in via `--kiwango-cha-jaribio <0-100>`) — minimum percentage of
   `#[jaribio]` test functions relative to public `kazi`. Not enforced unless the flag is passed.

## Not yet checked

**Type stability across versions** (breaking public-signature changes between releases) is not
implemented — it needs a baseline (a snapshot of a previous version's public API) to diff
against, and nothing in the toolchain publishes/tags versions yet. Planned approach: a git-tag
convention (`v<toleo>`) as the baseline source once `pata ongeza`'s dependency-fetch path (see
[pata-production-readiness.md](../design/pata-production-readiness.md)) is in place. See
[implementation-status.md](../design/implementation-status.md) for overall status.
