# `pata njozi`

Purpose: initialize a new Asili project with optional template selection.

Flags:
- `--kiasi` — binary (application) template; default. Creates `src/kuu.as` with `kuu(hoja: Orodha<Neno>)` entry point
- `--maktaba` — library template. Creates `src/kuu.as` with `example()` function
- `--workspace` — workspace template. Root `pata.toml` declares `[eneo-kazi] wanachama = ["core", "lib"]`
  (see `pata jenga --workspace-info`); each member (`core/`, `lib/`) is its own leaf project with its
  own `pata.toml`/`src/kuu.as`. No separate `Asili.toml` file — one manifest format for both leaf and
  workspace-root projects.

Inputs:
- optional project name (default: `asili-app`)
- optional target directory (default: `{project-name}`)

Success:
- `--kiasi`/`--maktaba`: creates `pata.toml`, `src/kuu.as` (template-specific), `.gitignore`,
  `kilele/.gitkeep`, `.github/workflows/ci.yml`
- `--workspace`: creates a root `pata.toml` ([eneo-kazi]) plus `core/` and `lib/` member projects,
  each with their own `pata.toml`/`src/kuu.as`

Failures:
- destination already exists and is non-empty
- insufficient filesystem permissions
- invalid project name (empty or whitespace)
