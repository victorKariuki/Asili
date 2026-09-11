# Formatting code

`pata nadhifu` auto-formats `.as`/`.asi` source files.

## Basic usage

From your project directory (where `pata.toml` lives):

```bash
pata nadhifu
```

Formats every `.as`/`.asi` file under the project in place and prints a summary:

```
imekamilika: mafaili 1 yalikaguliwa, 1 yamebadilishwa
```

## Check mode (for CI)

```bash
pata nadhifu --kagua
```

Doesn't write anything — exits 1 if any file isn't already in canonical form (and prints
`format haijafuata viwango: mafaili N/M`), exits 0 with `sawa: mafaili yote N yako katika
muundo sahihi` if everything's already formatted. Use this in CI instead of the write mode.

## Scoping to a path

```bash
pata nadhifu src/kuu.as
pata nadhifu src/
```

Only one path argument is accepted (`tolea njia moja tu` if you pass more than one); omit it
to format the whole project from the current directory.

## Known limitation

`pata nadhifu` is a **line-level text transform, not an AST-based formatter** — it doesn't
parse and re-print the source. This means it can misjudge indentation in some cases (e.g. it
may strip meaningful leading whitespace inside a function body on some inputs) and, per its own
source comments, can corrupt string literals containing `{`/`}`/`,` via blind brace/comma
replacement. Review the diff after running it, especially on files with such content. See
[implementation-status.md](../design/implementation-status.md) for status.

## Before committing

`pata thibitisha` (see [04-validate-docs.md](04-validate-docs.md)) requires the project to
already be in canonical format — run `pata nadhifu` first if `thibitisha` fails with `nadhifu
check imefeli`.
