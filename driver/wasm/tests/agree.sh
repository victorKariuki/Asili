#!/usr/bin/env bash
# The browser build runs every example exactly as native code does: each example is run by the
# native runner (`tenda`) and by asili-wasm under Node (native code as a wasm module, as in the
# playground), and the outputs must match. Needs the wasm32-unknown-unknown target, Node and a
# `wasm-bindgen` CLI of the version in Cargo.lock.
# Verifies: REQ-TGT-1
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

cargo build --release -q -p pata-cli -p asili-runner --manifest-path "$root/Cargo.toml"
cargo build --release -q -p asili-wasm --target wasm32-unknown-unknown --features wasm-browser \
  --manifest-path "$root/Cargo.toml"
wasm-bindgen --target nodejs --out-dir "$out/pkg" \
  "$root/target/wasm32-unknown-unknown/release/asili_wasm.wasm"

# Examples that need no network, files or input, then random programs from the optimizer's
# differential fuzzer (`core/evaluator/tests/fuzz.rs`; `ASILI_FUZZ_PROGRAMS`, default 100).
examples="all_types astar binary_ops control_structures data_structures kidhibiti namba_kuu phase1_modules
  seti sifa sudoku unary_ops"
ASILI_FUZZ_WRITE="$out/fuzz" ASILI_FUZZ_PROGRAMS="${ASILI_FUZZ_PROGRAMS:-100}" \
  cargo test --release -q -p asili-evaluator --test fuzz --manifest-path "$root/Cargo.toml" \
  write_programs_for_other_targets >/dev/null
fuzz=$(cd "$out/fuzz" && ls | sort -n | sed 's/^/fuzz:/')
status=0
for name in $examples $fuzz; do
  case $name in
    fuzz:*) dir="$out/fuzz/${name#fuzz:}" ;;
    *) dir="$root/examples/$name" ;;
  esac
  (cd "$dir" && "$root/target/release/pata-cli" jenga >/dev/null)
  "$root/target/release/tenda" "$dir"/kilele/*.asb >"$out/native.txt" 2>&1 || true
  node "$root/driver/wasm/tests/run_node.mjs" "$out/pkg" "$dir"/src/kuu.as >"$out/wasm.txt" 2>&1 || true
  if cmp -s "$out/native.txt" "$out/wasm.txt"; then
    echo "sawa: $name"
  else
    echo "TOFAUTI: $name"
    diff "$out/native.txt" "$out/wasm.txt" | head -20
    status=1
  fi
done
exit $status
