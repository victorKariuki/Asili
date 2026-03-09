# `pata njozi`

Purpose: initialize a new Asili project.

Inputs:
- optional project name
- optional target directory

Success:
- creates `pata.toml`, `src/kuu.as`, `.gitignore`, `target/.gitkeep`

Failures:
- destination already exists and is non-empty
- insufficient filesystem permissions
