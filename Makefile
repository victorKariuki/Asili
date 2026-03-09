# Asili workspace — build, test, install
# Requires: cargo (Rust toolchain)

.PHONY: build test install

build:
	cargo build

build-release:
	cargo build --release

test:
	cargo test

# Install pata and pata-lsp to DESTDIR (default: no prefix; copy to ~/.local/bin if desired)
# Usage: make install [DESTDIR=~/.local/bin]
DESTDIR ?= /usr/local/bin
install: build-release
	install -d "$(DESTDIR)"
	install -m 755 target/release/pata "$(DESTDIR)/pata"
	install -m 755 target/release/pata-lsp "$(DESTDIR)/pata-lsp"
