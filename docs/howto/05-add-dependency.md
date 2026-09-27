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

## Where the package comes from

| `pata ongeza` form | Source | What happens |
|---|---|---|
| `pata ongeza jitu --git <url> [--tawi <jina>]` | a git repository | cloned into `.asili/packages/jitu/`, content-hashed into `pata.lock` |
| `pata ongeza jitu --toleo "^1.0"` | a registry | the highest version satisfying every constraint (including those of transitive dependents) is fetched into `.asili/packages/jitu/` and content-hashed |
| a `path = "..."` entry written by hand in `[tegemezi]` | a local directory | used in place |

Registry lookups first use the local index at `.asili/registry/`. When that can't satisfy the
constraint, and a hosted index is configured, `pata` fetches `<index>/index/<name>/index.json`
from it, downloads the chosen version's `.tar.gz`, verifies its SHA-256 against the index before
extracting, and caches the index rows in `.asili/registry/` so later builds work offline:

```toml
# pata.toml
[rejista]
faharasa = "https://example.org/pata-index"
```

(`PATA_REJISTA=<url>` overrides it for one command.) An index row is
`{"version": "1.2.0", "checksum": "<sha256 of the tarball>", "url": "https://.../pkg-1.2.0.tar.gz", "deps": [{"name": "util", "req": "^0.3"}]}`
— a static file host or git repository is enough; there is no publish command or API server.
The `[tegemezi]` section format itself is defined in
[06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md).
