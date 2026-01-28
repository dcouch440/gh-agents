.PHONY: build check test fmt lint clean run release help

# Default target
help:
	@echo "Available targets:"
	@echo "  make build    - Build debug binary"
	@echo "  make release  - Build release binary"
	@echo "  make check    - Fast type checking"
	@echo "  make test     - Run all tests"
	@echo "  make fmt      - Format code"
	@echo "  make lint     - Run clippy linter"
	@echo "  make clean    - Remove build artifacts"
	@echo "  make run      - Run the application"
	@echo "  make watch    - Watch and rebuild on changes (requires cargo-watch)"
	@echo "  make all      - Format, lint, and test"

# Build
build:
	cargo build

release:
	cargo build --release

check:
	cargo check

# Testing
test:
	cargo test

test-verbose:
	cargo test -- --nocapture

# Code quality
fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

# Run
run:
	cargo run

run-debug:
	RUST_LOG=debug cargo run

# Watch mode (requires: cargo install cargo-watch)
watch:
	cargo watch -x check -x test

# Clean
clean:
	cargo clean

# Combined targets
all: fmt lint test

ci: check fmt lint test
