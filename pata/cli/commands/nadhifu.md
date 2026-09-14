# `pata nadhifu`

Purpose: apply canonical formatting to `.as` / `.asi`.

Inputs:
- optional path scope
- optional check-only mode (`--kagua`)
- optional diff mode (`--diff`): never writes, prints a unified diff of what would change

Success:
- updates files in place, or exits cleanly in check/diff mode

Failures:
- parse errors preventing safe formatting
- filesystem write errors
