# Tracing (Pata-Trace)

One tracing API for the compiler and the runtime (`core/trace`, crate `asili-trace`). Code that
wants to be observable calls only `emit(tukio, name, line)`, `enter(name, line)` (a span, ended
when its guard drops) or `phase(name)` (a build-phase span). Where events go is chosen once, at
start-up, by installing a sink; the hooks themselves never change.

## Turning it on

| Where | How |
|---|---|
| Any run — `tenda`, standalone executables, `pata` itself | `ASILI_FUATILIA=<namna>` |
| `pata jenga` (build, and the run with `--tenda`) | `--fuatilia[=<namna>]` |
| `pata tenda` | `--fuatilia[=<namna>]` before the artifact path |
| Reading a binary recording | `pata fuatilia <faili>` |

`<namna>` is `mti` (the default; to standard error), `mti:<faili>`, `json`, `json:<faili>` or
`binari:<faili>`. An unknown format fails with exit code 2 before anything runs.

## Events

| Id | `Tukio` | Meaning (as the tree shows it) | Emitted by |
|---|---|---|---|
| `0x01` | `Ingia` | *kuingia* — a span starts | `kazi` calls (tree-walker; native code's host), parser blocks and functions |
| `0x02` | `Toka` | *kutoka* — the innermost span ends | the same, when they finish |
| `0x03` | `Kigeuzi` | *kutenga nafasi ya kigeuzi* — a binding | `weka`/`thabiti` on the tree-walker |
| `0x04` | `Hesabu` | *operesheni ya hesabu* | reserved (per-operation events would drown the rest) |
| `0x05` | `MwitoMfumo` | *mwito wa mfumo* — a builtin call | the tree-walker; native code's host |
| `0x06` | `Kosa` | *kosa* — a runtime error | once, by the `kazi` the error first leaves |
| `0x07` | `Hatua` | *hatua ya ujenzi* — a build phase (a span) | `pata`: `uchanganuzi`, `utatuzi`, `semantiki`, `bytecode`, `msimbo asilia` |
| `0x08` | `Urejeshaji` | *urejeshaji baada ya kosa la sintaksia* | the parser's synchronize step |

Depth is per thread (threads started by `tenda`, `mkondo_tumikia` carry their own); the readable
tree prefixes other threads with `[uzi N]`.

## Outputs

- **`mti`** — an indented tree: `▶` span start (with its line), `◀` span end (with its duration),
  `◆` build phase, `•` event, `✖` error. Coloured when standard error is a terminal and
  `NO_COLOR` is unset.
- **`json`** — one JSON object per line in OpenTelemetry's shape: spans
  (`"kind":"span"`, `traceId`, `spanId`, `parentSpanId`, `name`, `startTimeUnixNano`,
  `endTimeUnixNano`, `attributes`) written when they end, and events (`"kind":"event"`, the
  enclosing `spanId`, `timeUnixNano`, `body`, `attributes`). Attributes are prefixed `asili.`
  (`mstari`, `uzi`, `kina`, `tukio`).
- **`binari`** — fixed 4-byte frames, no text:

  | Byte 0 | Byte 1 | Bytes 2–3 |
  |---|---|---|
  | event id | depth (saturating at 255) | source line, big-endian (saturating at 65535) |

  `pata fuatilia` (`asili_trace::decode`) prints a recording as the Swahili tree
  (`└── tukio: kuingia — mstari 7`). A trailing partial frame or an unknown id is reported, not
  fatal.

## Cost and limits

- Off (the default), every hook is one relaxed atomic load (`asili_trace::on()`); spans store
  nothing. Measured against the commit before tracing: every benchmark within noise, native and
  tree-walker.
- Native code carries no hooks. Calls through native code's host (every call from the tree-walker,
  calls with list or generic arguments, deep calls) are traced; direct native-to-native calls
  are not, and native code has no line numbers (they show as 0).
- Binary frames are what a microcontroller target would emit over a serial line, but Asili has no
  bare-metal target yet: today the format is a compact recording for files.
- `toka` (which ends the process) flushes the trace first; so do the runner and `pata` on every
  exit path.
