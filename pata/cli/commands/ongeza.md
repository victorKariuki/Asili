# `pata ongeza <lib>`

Purpose: add a dependency to `pata.toml`.

Inputs:
- required library identifier
- optional version constraint

Success:
- writes `[tegemezi]` entry in `pata.toml`
- updates `pata.lock`

Failures:
- malformed library/version syntax
- resolver conflict
- network/index unavailable (when remote resolution is required)
