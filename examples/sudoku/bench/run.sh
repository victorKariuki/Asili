#!/usr/bin/env bash
# Benchmark the Asili Sudoku solver against identical C, Rust and Python solvers.
# Usage: ./run.sh [runs]   (default 5; prints the best wall time per implementation)
#
# Asili tiers run on the standalone runner (`tenda`, what a deployment ships):
# asili-nguvu = native code built in-house by `pata jenga` (no external tools),
# asili-vm = register-VM interpreter (ASILI_AOT=0). clang is only used here to build the
# C comparison, when installed.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
runs="${1:-5}"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

cargo build --release -q -p pata-cli -p asili-runner --manifest-path "$root/Cargo.toml"
# release: fail instead of silently benchmarking the VM when native code cannot be built.
(cd "$here/.." && "$root/target/release/pata-cli" jenga --namna release >/dev/null)
gcc -O2 -o "$out/sudoku_c" "$here/sudoku.c"
command -v clang >/dev/null && clang -O2 -o "$out/sudoku_clang" "$here/sudoku.c"
rustc -O -o "$out/sudoku_rs" "$here/rust/main.rs"

best() {
  local label="$1"; shift
  local min="" expected="Majaribio: 90665"
  for _ in $(seq "$runs"); do
    local start end ms output
    start=$(date +%s%N)
    output="$("$@")"
    end=$(date +%s%N)
    grep -q "$expected" <<<"$output" || { echo "$label: wrong result" >&2; echo "$output" >&2; exit 1; }
    ms=$(( (end - start) / 100000 ))  # tenths of a millisecond
    if [[ -z "$min" || "$ms" -lt "$min" ]]; then min="$ms"; fi
  done
  printf '%-12s %6d.%d ms\n' "$label" $((min / 10)) $((min % 10))
}

best c-gcc "$out/sudoku_c"
[[ -x "$out/sudoku_clang" ]] && best c-clang "$out/sudoku_clang"
best rust "$out/sudoku_rs"
command -v python3 >/dev/null && best python python3 "$here/sudoku.py"
asb="$here/../kilele/sudoku.asb"
runner="$root/target/release/tenda"
best asili-nguvu "$runner" "$asb"
ASILI_AOT=0 best asili-vm "$runner" "$asb"
