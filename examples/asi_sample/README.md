# asi_sample

Example project that uses **.asi** (interface) modules, builds, and runs.

- **Interfaces:** Imports `leta mfumo` and `leta hisabati` (stdlib interfaces from the registry).
- **Build:** From this directory run `pata jenga` (or from repo root: `pata jenga -C examples/asi_sample` if supported).
- **Run:** `pata jenga --tenda` builds and executes `kazi kuu`.

```bash
cd examples/asi_sample
cargo run -p pata-cli -- jenga --tenda
```

Output: prints `jumla(2, 3) = 5` and `duara(4) = 12.566...` using the hisabati and mfumo interfaces.

**Stream semantics (mfumo):** `chapisha` writes to **stdout** (result only); `onyo` and `makosa` write to **stderr** (warning and error telemetry). When piping (e.g. `pata jenga --tenda > out.txt`), only `chapisha` output goes to the file; diagnostics stay on the terminal.
