# REPL topic docs

Use `?topic` inside the REPL to show help (e.g. `?hisabati`, `?orodha`).

All 16 topic files exist today (`?topic` on a missing topic just fails to find the file —
there's no placeholder).

| Topic         | File              | Description                      |
|---------------|-------------------|-----------------------------------|
| `aina`        | aina.md           | Types, casting, special values   |
| `faili`       | faili.md          | Faili, Mkondo, Kumbukumbu (resource handles) |
| `hisabati`    | hisabati.md       | Math module functions            |
| `namba-kuu-sahihi` | namba-kuu-sahihi.md | Namba_Kuu and Namba_Sahihi (arbitrary precision) |
| `jozi`        | jozi.md           | Pair creation and methods        |
| `kamusi`      | kamusi.md         | Dictionary creation and methods  |
| `kazi`        | kazi.md           | Expressions/statements in the REPL (functions can't be defined interactively — see below) |
| `makosa`      | makosa.md         | Chaguo, Tokeo, error handling    |
| `neno`        | neno.md           | String methods and operations    |
| `orodha`      | orodha.md         | List creation and methods        |
| `udhibiti`    | udhibiti.md       | Control flow in the REPL         |
| `seti`        | seti.md           | Set creation and methods         |
| `sambamba`    | sambamba.md       | tenda/njia/fungo (concurrency, defined in a file) |
| `sifa`        | sifa.md           | Traits (defined in a file, not interactively — see below) |
| `umbo`        | umbo.md           | Structs (defined in a file, not interactively — see below) |
| `waendeshaji` | waendeshaji.md    | Operators, precedence, bitwise   |

## Starting the REPL

```bash
pata repl
```

Output:
```
Asili REPL. Andika 'toka' au 'exit' kuondoka. ?mada = msaada (mf. ?hisabati). ?lugha en/sw = badilisha lugha ya msaada.
>
```

## REPL commands

| Command           | Description                           |
|-------------------|----------------------------------------|
| `?topic`          | Show help for a topic (e.g. `?orodha`) |
| `?lugha en/sw`    | Switch help-doc language (English/Swahili) |
| `toka` / `exit`   | Quit the REPL                          |
| Any expression    | Evaluate and print the result          |
| Any statement     | Execute (e.g. `weka x = 5`)            |

## Example session

```
> weka x = 10
> weka y = 25
> x + y
Namba(35.0)
> "Jibu ni: " + ((x + y) kama Neno)
Neno("Jibu ni: 35")
> ?hisabati
(shows hisabati.md)
> toka
```

## Features

- State persists across lines (variables stay defined)
- Bare expressions are automatically returned and printed
- `?topic` reads `docs/repl/{lang}/{topic}.md` directly on demand — no fixed list, no
  directory scan, so a missing file just fails to open
- Errors are shown inline without crashing the session
- **Most top-level items cannot be defined interactively.** Every line is evaluated as if it
  were one statement inside a function body — so `kazi` (functions), `umbo` (structs),
  `shughuli ya` (impl blocks), `jenum`, `sifa`, and `leta` all fail to parse at the `>` prompt.
  `thabiti` is the exception — it behaves like `weka` (a plain statement) and works fine. The
  REPL is for evaluating expressions/statements against already-defined builtins; write new
  functions/structs/modules in a `.as` file and run with `pata jenga --tenda`. See
  [?kazi](kazi.md) and [?umbo](umbo.md).
- A line that recurses (evaluating any expression counts) prints a trailing `  (undani: N)`
  line showing the evaluation's peak call depth — this appears after almost every real
  evaluation but is omitted from the example transcripts above and in the other topic docs for
  readability; expect to see it in practice.
