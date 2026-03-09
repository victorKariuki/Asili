# `pata nadhifu`

Purpose: apply canonical formatting to `.as` / `.asi`.

Inputs:
- optional path scope
- optional check-only mode

Success:
- updates files in place, or exits cleanly in check mode

Failures:
- parse errors preventing safe formatting
- filesystem write errors
