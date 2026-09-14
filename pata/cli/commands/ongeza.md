# `pata ongeza <lib>`

Purpose: add a dependency to `pata.toml`.

Inputs:
- required library identifier
- `--toleo <semver>` optional version constraint (default `^0.1`)
- `--git <url>` optional: fetch real source via `git2` clone into `.asili/packages/<lib>/`,
  computing a real SHA-256 content hash over the fetched tree and writing it into `pata.lock`
  (`source = "git"`). `.git/` metadata is stripped from the vendored copy.
- `--tawi <jina>` optional: git branch to clone when `--git` is given (defaults to the repo's
  default branch)

Success:
- writes `[tegemezi]` entry in `pata.toml`
- updates `pata.lock`
- with `--git`: vendored source exists under `.asili/packages/<lib>/`, and `pata.lock`'s
  checksum for that dependency is a real 64-hex-char SHA-256 digest over the fetched tree, not a
  hash of the name/version string

Failures:
- malformed library/version syntax
- with `--git`: clone failure (bad URL, unreachable host, nonexistent branch) fails the command
  with a Swahili error naming the lib and URL
- plain version dependencies (no `--git`) still only write to `pata.toml`/`pata.lock` with no
  fetch — there is no registry backend yet to resolve them against (see
  `docs/design/pata-production-readiness.md` item 3 / `pata-implementation-spec.md` Section 16)
