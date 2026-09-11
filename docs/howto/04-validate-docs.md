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
2. **Formatting** — the project must already be in canonical `pata nadhifu` form; run `pata
   nadhifu` first if you see `nadhifu check imefeli: tumia \`pata nadhifu\` kwanza`.
3. **Public documentation coverage** — every `umma kazi` (public function) needs a `///` doc
   comment on the line directly above it:

   ```asili
   /// Adds two numbers.
   umma kazi jumla(a: Namba, b: Namba) -> Namba {
     rejesha a + b
   }
   ```

   Missing one fails with `nyaraka zimekosekana kwa kazi ya umma '<name>' kwenye <file>:<line>`.

## Not yet checked

Per the tool's own TODO, `thibitisha` does not yet check: type stability across versions, ABI
compatibility for `#[kiunganishi]` (FFI) exports, trait-completeness (every `sifa` implemented
by declared dependencies), or a test-coverage threshold. See
[implementation-status.md](../design/implementation-status.md) for status.
