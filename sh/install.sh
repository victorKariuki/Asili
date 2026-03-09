#!/usr/bin/env bash
# Build and install pata + pata-lsp to a directory on PATH.
# Usage: ./sh/install.sh [install-dir]
# Default install-dir: ~/.local/bin

set -e
INSTALL_DIR="${1:-$HOME/.local/bin}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$REPO_ROOT"
cargo build --release
mkdir -p "$INSTALL_DIR"
install -m 755 target/release/pata "$INSTALL_DIR/pata"
install -m 755 target/release/pata-lsp "$INSTALL_DIR/pata-lsp"
echo "Installed pata and pata-lsp to $INSTALL_DIR"
echo "Ensure $INSTALL_DIR is on your PATH."
