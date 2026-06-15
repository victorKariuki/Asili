# REPL topic docs

Use `?mada` inside the REPL to show help (e.g. `?hisabati`, `?orodha`).

| Mada          | Faili             | Maelezo                          |
|---------------|-------------------|----------------------------------|
| `aina`        | aina.md           | Types, casting, special values   |
| `hisabati`    | hisabati.md       | All 46 math module functions     |
| `jozi`        | jozi.md           | Pair creation and methods        |
| `kamusi`      | kamusi.md         | Dictionary creation and methods  |
| `kazi`        | kazi.md           | Defining functions in the REPL   |
| `makosa`      | makosa.md         | Chaguo, Tokeo, error handling    |
| `neno`        | neno.md           | String methods and operations    |
| `orodha`      | orodha.md         | List creation and methods        |
| `udhibiti`    | udhibiti.md       | Control flow in the REPL         |
| `umbo`        | umbo.md           | Struct definition and methods    |
| `waendeshaji` | waendeshaji.md    | Operators, precedence, bitwise   |

## Kuanza REPL

```bash
pata repl
```

Output:
```
Asili REPL. Andika 'toka' au 'exit' kuondoka. ?mada = msaada (mf. ?hisabati).
>
```

## Amri za REPL

| Amri              | Maelezo                              |
|-------------------|--------------------------------------|
| `?mada`           | Onyesha msaada wa mada (e.g. `?orodha`) |
| `toka` / `exit`   | Funga REPL                          |
| Any expression    | Evaluate and print the result        |
| Any statement     | Execute (e.g. `weka x = 5`)         |

## Mfano wa Kikao

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

## Vipengele

- State persists across lines (variables stay defined)
- Bare expressions are automatically returned and printed
- `?topic` reads from `docs/repl/{topic}.md` relative to project root
- Errors are shown inline without crashing the session
