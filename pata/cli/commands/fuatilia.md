# `pata fuatilia`

Purpose: show a binary trace recording as the readable tree.

Usage: `pata fuatilia <faili>`

- reads 4-byte frames written by `--fuatilia=binari:<faili>` or `ASILI_FUATILIA=binari:<faili>`
  (see `docs/design/tracing.md`) and prints one line per frame, indented by depth:
  `└── tukio: kuingia — mstari 7`
- an unknown event id prints `tukio lisilojulikana 0x..`; a trailing partial frame prints
  `fremu isiyokamilika mwishoni mwa faili`
- fails (exit 1) with `imeshindwa kusoma <faili>: ...` when the file cannot be read, and (exit 2)
  when not given exactly one file
