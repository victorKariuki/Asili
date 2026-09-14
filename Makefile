# Asili workspace — build, test, install
# Requires: cargo (Rust toolchain)

.PHONY: build test install install-ext

build:
	cargo build

build-release:
	cargo build --release

test:
	cargo test

# Install pata, pata-lsp, and pata-lint to DESTDIR (default: no prefix; copy to ~/.local/bin if desired)
# Usage: make install [DESTDIR=~/.local/bin]
DESTDIR ?= /usr/local/bin
install: build-release
	install -d "$(DESTDIR)"
	install -m 755 target/release/pata-cli "$(DESTDIR)/pata"
	install -m 755 target/release/pata-lsp "$(DESTDIR)/pata-lsp"
	install -m 755 target/release/pata-lint "$(DESTDIR)/pata-lint"

# Build the VSCode extension (bundles pata-lsp + its own TS sources into a .vsix) and
# install it into VS Code. Requires `code` on PATH and npm/npx.
install-ext: build-release
	mkdir -p extensions/vscode/bin
	cp target/release/pata-lsp extensions/vscode/bin/pata-lsp
	cd extensions/vscode && npm install && npx --yes @vscode/vsce package --no-dependencies
	code --install-extension extensions/vscode/asili-*.vsix
