# `pata ongeza <lib>`

Purpose: add a dependency to `pata.toml`, resolved for real.

Inputs:
- required library identifier
- `--toleo <semver>` optional version constraint (default `^0.1`)
- `--git <url>` optional: fetch real source via `git2` clone into `.asili/packages/<lib>/`,
  computing a real SHA-256 content hash over the fetched tree and writing it into `pata.lock`
  (`source = "git"`). `.git/` metadata is stripped from the vendored copy. The manifest records
  `{ git = "<url>", version = "<constraint>" }`, not a bare version string, so later resolves
  correctly treat this as a git dependency.
- `--tawi <jina>` optional: git branch to clone when `--git` is given (defaults to the repo's
  default branch)

Resolution (no `--git`): a bare-version dependency resolves against the local registry index at
`.asili/registry/` (one JSON file per package, `<name>.json`, each entry a published version plus
a fetchable `RegistrySource` — git URL or filesystem path; see `pata_package::registry`). The
highest version satisfying the given constraint is selected, fetched for real into
`.asili/packages/<lib>/`, and content-hashed — not a placeholder. A constraint with no matching
published version, or no registry index at all, fails the resolve rather than silently
succeeding. An already-locked version that still satisfies the constraint is kept rather than
re-resolved, avoiding unnecessary re-fetches on every build.

Success:
- writes `[tegemezi]` entry in `pata.toml`
- updates `pata.lock` with a real, concrete resolved version and a real SHA-256 content checksum
  (64 hex chars) — never a hash of the name/version string, for any source (`path` dependencies
  are the sole exception, having no fetchable content of their own)
- with `--git`: vendored source exists under `.asili/packages/<lib>/`, a `.pata-version` marker
  records the fetched version for future resolves

Failures:
- malformed library/version syntax
- with `--git`: clone failure (bad URL, unreachable host, nonexistent branch) fails the command
  with a Swahili error naming the lib and URL
- no `--git`: no registry entry satisfying the given version constraint fails the resolve with a
  Swahili error naming the constraint and registry path searched
