# `pata njozi`

Purpose: initialize a new Asili project with optional template selection.

Flags:
- `--kiasi` — binary (application) template; default. Creates `src/kuu.as` with `kuu(hoja: Orodha<Neno>)` entry point
- `--maktaba` — library template. Creates `src/kuu.as` with `example()` function

Inputs:
- optional project name (default: `asili-app`)
- optional target directory (default: `{project-name}`)

Success:
- creates `pata.toml`, `src/kuu.as` (template-specific), `.gitignore`, `kilele/.gitkeep`, `.github/workflows/ci.yml`

Failures:
- destination already exists and is non-empty
- insufficient filesystem permissions
- invalid project name (empty or whitespace)
