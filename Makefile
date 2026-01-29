.PHONY: build check test fmt lint clean run release help \
       ui-install ui-dev ui-build ui-lint ui-preview \
       dev build-all lint-all test-all ci

# Default target
help:
	@echo "Available targets:"
	@echo ""
	@echo "  Backend (Rust):"
	@echo "    make build       - Build debug binary"
	@echo "    make release     - Build release binary"
	@echo "    make check       - Fast type checking"
	@echo "    make test        - Run all tests"
	@echo "    make fmt         - Format code"
	@echo "    make lint        - Run clippy linter"
	@echo "    make run         - Run the application"
	@echo "    make run-debug   - Run with debug logging"
	@echo "    make watch       - Watch and rebuild on changes"
	@echo ""
	@echo "  Frontend (React):"
	@echo "    make ui-install  - Install npm dependencies"
	@echo "    make ui-dev      - Start Vite dev server"
	@echo "    make ui-build    - Build frontend for production"
	@echo "    make ui-lint     - Run eslint"
	@echo "    make ui-preview  - Preview production build"
	@echo ""
	@echo "  Combined:"
	@echo "    make dev         - Run backend and frontend dev servers"
	@echo "    make build-all   - Build backend and frontend"
	@echo "    make lint-all    - Lint backend and frontend"
	@echo "    make test-all    - Run all tests"
	@echo "    make ci          - Full CI check (fmt, lint, test, ui-build)"
	@echo "    make clean       - Remove all build artifacts"

# --- Backend (Rust) ---

build:
	cargo build

release:
	cargo build --release

check:
	cargo check

test:
	cargo test

test-verbose:
	cargo test -- --nocapture

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

run:
	cargo run

run-debug:
	RUST_LOG=debug cargo run

watch:
	cargo watch -x check -x test

# --- Frontend (React) ---

ui-install:
	cd ui && npm install

ui-dev:
	cd ui && npm run dev

ui-build:
	cd ui && npm run build

ui-lint:
	cd ui && npm run lint

ui-preview:
	cd ui && npm run preview

# --- Combined ---

dev:
	@echo "Starting backend and frontend..."
	@trap 'kill 0' INT; \
		cargo run & \
		(cd ui && npm run dev) & \
		wait

build-all: build ui-build

lint-all: lint ui-lint

test-all: test

ci: check fmt lint test ui-build

clean:
	cargo clean
	rm -rf ui/dist ui/node_modules/.vite
