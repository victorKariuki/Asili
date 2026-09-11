# Adding a dependency

`pata ongeza <lib>` adds an entry to `pata.toml`'s `[tegemezi]` table and updates `pata.lock`.

## Basic usage

```bash
pata ongeza jitu
```

```
tegemezi 'jitu' imeongezwa
imekamilika: tegemezi 'jitu' = '^0.1'
```

Adds (or updates) a `[tegemezi]` entry in `pata.toml` and regenerates `pata.lock` (records a
SHA-256 checksum for the resolved dependency).

## Pinning a version

```bash
pata ongeza jitu --toleo "^1.0"
```

`--toleo` (Swahili for "version") accepts a semver-like constraint string. Without it, the
default constraint is `^0.1`.

## What this does NOT do

`pata ongeza` writes the manifest/lockfile entries but does **not** fetch, download, or resolve
the package from anywhere — there is no package registry or network resolver in this toolchain
yet. It's for recording a dependency you're providing another way (a path dependency, or a
locally-vendored package). See
[implementation-status.md](../design/implementation-status.md) for current package-manager
scope, and the `[tegemezi]` section format itself is defined in
[06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md).
