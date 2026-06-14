# Getting started with Asili

## Prerequisites

- **Rust toolchain** — e.g. [rustup](https://rustup.rs). Asili’s compiler and tools are written in Rust.

## Build

From the **workspace root** (the Asili repo root):

```bash
cargo build
```

This builds the `pata` CLI and the `pata-lsp` binary. The `pata` binary is produced in `target/debug/pata` (or `target/release/pata` after `cargo build --release`).

To build an **Asili project** (a directory with `pata.toml` and entrypoint), run from that directory:

```bash
pata jenga
```

This compiles the entrypoint (e.g. `src/kuu.as`) and produces bytecode/artifacts.

## Run one file

From an Asili project directory (with `pata.toml` and e.g. `src/kuu.as`):

```bash
pata jenga --tenda
```

This compiles and executes the entrypoint. You can pass arguments after `--tenda`; they are available to `kazi kuu(hoja: Orodha<Neno>)` as `hoja`.

## Run tests

From an Asili project directory:

```bash
pata jaribu
```

This discovers all `#[jaribio]` functions and runs them. Use `pata jaribu --list` to list test names without running them.

## New project

From an **empty directory**:

```bash
pata njozi
```

This creates `pata.toml` and `src/kuu.as` so you can start writing Asili code. Then run `pata jenga --tenda` or `pata jaribu` as above.

## Install (optional)

To use `pata` from anywhere:

1. **Makefile:** From the repo root, run `make install` (installs to `/usr/local/bin` by default). Use `make install DESTDIR=~/.local/bin` to install to a user directory.
2. **Script:** Run `./sh/install.sh` to build and install `pata` and `pata-lsp` into `~/.local/bin`. Or pass a directory: `./sh/install.sh /path/to/bin`.
3. **Manual:** Build with `cargo build --release`, then copy `target/release/pata` and `target/release/pata-lsp` to a directory on your `PATH`.

See [spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) for the full Pata command reference.
