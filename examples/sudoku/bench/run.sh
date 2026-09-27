#!/usr/bin/env bash
# Benchmark the Asili Sudoku solver against identical C, Rust and Python solvers.
# Usage: ./run.sh [runs]   (default 5; prints the best wall time per implementation)
#
# Asili tiers: asili-aot = LLVM native library built by `pata jenga` (needs clang),
# asili-jit = Cranelift JIT (ASILI_AOT=0), asili-vm = register-VM interpreter (also ASILI_JIT=0).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
runs="${1:-5}"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

cargo build --release -q -p pata-cli --manifest-path "$root/Cargo.toml"
(cd "$here/.." && "$root/target/release/pata-cli" jenga >/dev/null)
gcc -O2 -o "$out/sudoku_c" "$here/sudoku.c"
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
    ms=$(( (end - start) / 1000000 ))
    if [[ -z "$min" || "$ms" -lt "$min" ]]; then min="$ms"; fi
  done
  printf '%-10s %8d ms\n' "$label" "$min"
}

best c "$out/sudoku_c"
best rust "$out/sudoku_rs"
command -v python3 >/dev/null && best python python3 "$here/sudoku.py"
asb="$here/../kilele/sudoku.asb"
best asili-aot "$root/target/release/pata-cli" tenda "$asb"
ASILI_AOT=0 best asili-jit "$root/target/release/pata-cli" tenda "$asb"
ASILI_AOT=0 ASILI_JIT=0 best asili-vm "$root/target/release/pata-cli" tenda "$asb"
