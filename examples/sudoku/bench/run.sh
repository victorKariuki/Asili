#!/usr/bin/env bash
# Benchmark the Asili Sudoku solver against identical C, Rust and Python solvers.
# Usage: ./run.sh [runs]   (default 5; prints the best wall time per implementation)
#
# Asili runs on the standalone runner (`tenda`, built as it ships — see below):
# asili-nguvu = native code built in-house by `pata jenga` (no external tools).
# clang is only used here to build the C comparison, when installed.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
runs="${1:-5}"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

cargo build --release -q -p pata-cli --manifest-path "$root/Cargo.toml"
# The runner as it ships: `dist` profile, statically linked against musl when that target is
# installed (`rustup target add x86_64-unknown-linux-musl`), else for the host.
musl="$(uname -m)-unknown-linux-musl"
if rustup target list --installed 2>/dev/null | grep -qx "$musl"; then
  cargo build --profile dist -q -p asili-runner --target "$musl" --manifest-path "$root/Cargo.toml"
  runner="$root/target/$musl/dist/tenda"
else
  cargo build --profile dist -q -p asili-runner --manifest-path "$root/Cargo.toml"
  runner="$root/target/dist/tenda"
fi
# release: fail when native code cannot be built.
(cd "$here/.." && "$root/target/release/pata-cli" jenga --namna release >/dev/null)
gcc -O2 -o "$out/sudoku_c" "$here/sudoku.c"
command -v clang >/dev/null && clang -O2 -o "$out/sudoku_clang" "$here/sudoku.c"
# Statically linked C too, for a like-for-like start-up comparison with the static runner.
gcc -O2 -static -o "$out/sudoku_c_static" "$here/sudoku.c" 2>/dev/null || true
command -v clang >/dev/null && { clang -O2 -static -o "$out/sudoku_clang_static" "$here/sudoku.c" 2>/dev/null || true; }
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
[[ -x "$out/sudoku_c_static" ]] && best c-gcc-st "$out/sudoku_c_static"
[[ -x "$out/sudoku_clang_static" ]] && best c-clang-st "$out/sudoku_clang_static"
best rust "$out/sudoku_rs"
command -v python3 >/dev/null && best python python3 "$here/sudoku.py"
asb="$here/../kilele/sudoku.asb"
best asili-nguvu "$runner" "$asb"
