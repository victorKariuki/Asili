# Running tests

Asili projects use **`#[jaribio]`** to mark test functions. The test runner discovers and runs them.

## Basic usage

From your project directory (where `pata.toml` lives):

```bash
pata jaribu
```

- **Exit 0** — All tests passed.
- **Exit 1** — One or more tests failed.
- **Exit 2** — Usage or config error (e.g. invalid `--chuja`).

## Options

- **`pata jaribu --orodha`** — List test names (one per line), do not run. Exit 0 if discovery succeeds.
- **`pata jaribu --chuja <pattern>`** — Run only tests whose name contains `pattern`. Also applies when using `--orodha`.
- **`pata jaribu --simama-haraka`** — Stop on first failure.

## Writing tests

Mark a function with the `#[jaribio]` attribute:

```asili
#[jaribio]
kazi jaribu_jumla() -> Tupu {
  weka j = jumla(2, 3)
  ikiwa j != 5 { paparika("jumla inashindwa") }
  rejesha
}
```

Tests take no arguments. Use `paparika` to fail with a message, or `rejesha` to pass.

## Summary output

After a run, the runner prints a line like:

```
jumla: N | sawa: M | kosa: K
```

Then either “majaribio yote yamefaulu” or “majaribio K yameshindwa”.

See [spec/06-tooling-and-ecosystem.md](spec/06-tooling-and-ecosystem.md) and the project’s `pata/cli/commands/jaribu.md` for details.
