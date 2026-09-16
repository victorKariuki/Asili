#!/usr/bin/env bash
# Builds the asili-wasm (wasm-browser) package used by examples/playground/main.js.
# Run from anywhere; writes output into examples/playground/pkg (gitignored).
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

command -v wasm-pack >/dev/null 2>&1 || {
  echo "wasm-pack haipatikani. Sakinisha kwa: cargo install wasm-pack" >&2
  exit 1
}

wasm-pack build "$repo_root/driver/wasm" \
  --target web \
  --out-dir "$script_dir/pkg" \
  --no-typescript \
  -- --no-default-features --features wasm-browser

# main.js fetches the wasm module as base64 text (not the raw .wasm binary) so it can be served
# by the Asili static server in src/kuu.as, whose soma_faili only reads valid-UTF-8 text files.
base64 -w0 "$script_dir/pkg/asili_wasm_bg.wasm" > "$script_dir/pkg/asili_wasm_bg.wasm.b64"

echo
echo "Imekamilika. Anza seva kwa mojawapo ya njia hizi:"
echo
echo "  # Asili yenyewe (mfano wa mkondo_tumikia_http, ona src/kuu.as):"
echo "  cd \"$script_dir\" && pata jenga --tenda"
echo
echo "  # au seva tuli yoyote, mfano:"
echo "  cd \"$script_dir\" && python3 -m http.server 8080"
echo
echo "kisha fungua http://127.0.0.1:8080/ (pata jenga --tenda) au http://localhost:8080/ (python)"
