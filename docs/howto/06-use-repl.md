# Using the REPL

```bash
pata repl
```

```
Asili REPL. Andika 'toka' au 'exit' kuondoka. ?mada = msaada (mf. ?hisabati). ?lugha en/sw = badilisha lugha ya msaada.
>
```

Type an expression or statement, press enter, see the result. State persists across lines.

```
> weka x = 10
> weka y = 25
> x + y
Namba(35.0)
  (undani: 2)
```

`toka` or `exit` quits. `?mada` shows inline help for a topic (`?hisabati`, `?orodha`, etc.) —
see [docs/repl/en/](../repl/en/) (English) or [docs/repl/sw/](../repl/sw/) (Swahili) for the
full topic list; switch languages inside the REPL with `?lugha en` / `?lugha sw`.

## The one thing that doesn't work: defining things

**Most top-level items — `kazi` (functions), `umbo` (structs), `shughuli ya` (impl blocks),
`jenum`, `sifa`, `leta` — cannot be typed at the REPL prompt.** Every line you type is
evaluated as if it were a single statement inside a function body; top-level items aren't valid
there and fail to parse. `thabiti` is the one exception — it behaves like `weka` (a plain
statement) and works fine at the REPL. There's no multi-line or file-load mode that works
around the rest.

The REPL is for evaluating expressions and simple statements (`weka`, `ikiwa`, loops,
`linganisha`) against **already-defined** builtins — not for interactively writing new
functions, structs, or modules. Write those in a `.as` file and run with `pata jenga --tenda`
(see [00-getting-started.md](00-getting-started.md)).

See [docs/repl/en/kazi.md](../repl/en/kazi.md) and [docs/repl/en/umbo.md](../repl/en/umbo.md)
for more.
