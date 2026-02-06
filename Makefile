.PHONY: build check test fmt lint clean run run-opus run-haiku release help sync \
       ui-install ui-dev ui-build ui-lint ui-preview \
       cli-install cli-dev cli-build \
       dev build-all lint-all test-all ci \
       report-last-run report-token-usage report-tool-calls report-sessions report-all

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
	@echo "    make run         - Run the application (Sonnet)"
	@echo "    make run-opus    - Run with Opus model"
	@echo "    make run-haiku   - Run with Haiku model (cheap)"
	@echo "    make run-debug   - Run with debug logging"
	@echo "    make watch       - Watch and rebuild on changes"
	@echo "    make sync        - Sync config files to database"
	@echo ""
	@echo "  Frontend (React):"
	@echo "    make ui-install  - Install npm dependencies"
	@echo "    make ui-dev      - Start Vite dev server"
	@echo "    make ui-build    - Build frontend for production"
	@echo "    make ui-lint     - Run eslint"
	@echo "    make ui-preview  - Preview production build"
	@echo ""
	@echo "  CLI (Terminal):"
	@echo "    make cli-install - Install CLI npm dependencies"
	@echo "    make cli-dev     - Launch the terminal CLI"
	@echo "    make cli-build   - Compile CLI TypeScript"
	@echo ""
	@echo "  Combined:"
	@echo "    make dev         - Run backend and frontend dev servers"
	@echo "    make build-all   - Build backend, frontend, and CLI"
	@echo "    make lint-all    - Lint backend and frontend"
	@echo "    make test-all    - Run all tests"
	@echo "    make ci          - Full CI check (fmt, lint, test, ui-build, cli-build)"
	@echo "    make clean       - Remove all build artifacts"
	@echo ""
	@echo "  Reports:"
	@echo "    make report-last-run    - Export last session's LLM rounds + tool calls to CSV"
	@echo "    make report-token-usage - Export token usage (last 24h) to CSV"
	@echo "    make report-tool-calls  - Export all tool calls (last 24h) to CSV"
	@echo "    make report-sessions    - Export session summary to CSV"
	@echo "    make report-all         - Generate all reports"

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

run-opus:
	ANTHROPIC_MODEL=claude-opus-4-20250514 cargo run

run-haiku:
	ANTHROPIC_MODEL=claude-3-5-haiku-20241022 cargo run

watch:
	cargo watch -x check -x test

sync:
	cargo run -- sync-config

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

# --- CLI (Terminal) ---

cli-install:
	cd cli && npm install

cli-dev:
	cd cli && npm run dev

cli-build:
	cd cli && npm run build

# --- Combined ---

dev:
	@echo "Starting backend and frontend..."
	@trap 'kill 0' INT; \
		cargo run & \
		(cd ui && npm run dev) & \
		wait

build-all: build ui-build cli-build

lint-all: lint ui-lint

test-all: test

ci: check fmt lint test ui-build cli-build

clean:
	cargo clean
	rm -rf ui/dist ui/node_modules/.vite
	rm -rf cli/dist cli/node_modules/.cache

# --- Reports ---

PSQL=docker exec -i $$(docker ps -q --filter "publish=5432" | head -1) psql -U nexor -d nexor
TIMESTAMP=$$(date +%Y%m%d-%H%M%S)

report-last-run:
	@echo "Exporting last run report..."
	@$(PSQL) -c "COPY ( \
		SELECT tu.created_at, tu.tier, tu.model_id, tu.input_tokens, tu.output_tokens, \
		       cs.title as session_title \
		FROM token_usage tu \
		LEFT JOIN chat_sessions cs ON tu.session_id = cs.id \
		WHERE tu.session_id = ( \
			SELECT session_id FROM token_usage ORDER BY created_at DESC LIMIT 1 \
		) \
		ORDER BY tu.created_at \
	) TO STDOUT WITH CSV HEADER" > reports/last-run-tokens-$(TIMESTAMP).csv
	@$(PSQL) -c "COPY ( \
		SELECT tc.created_at, tc.round, tc.tool_name, tc.tool_use_id, \
		       tc.input::text as input, \
		       LEFT(tc.output, 500) as output_preview, \
		       tc.latency_ms \
		FROM tool_calls tc \
		WHERE tc.session_id = ( \
			SELECT session_id FROM token_usage ORDER BY created_at DESC LIMIT 1 \
		) \
		ORDER BY tc.created_at \
	) TO STDOUT WITH CSV HEADER" > reports/last-run-tools-$(TIMESTAMP).csv
	@echo "Saved to reports/last-run-tokens-$(TIMESTAMP).csv"
	@echo "Saved to reports/last-run-tools-$(TIMESTAMP).csv"

report-token-usage:
	@echo "Exporting token usage (last 24h)..."
	@$(PSQL) -c "COPY ( \
		SELECT tu.created_at, tu.session_id, tu.tier, tu.model_id, \
		       tu.input_tokens, tu.output_tokens, \
		       (tu.input_tokens + tu.output_tokens) as total_tokens \
		FROM token_usage tu \
		WHERE tu.created_at > NOW() - INTERVAL '24 hours' \
		ORDER BY tu.created_at \
	) TO STDOUT WITH CSV HEADER" > reports/token-usage-$(TIMESTAMP).csv
	@echo "Saved to reports/token-usage-$(TIMESTAMP).csv"

report-tool-calls:
	@echo "Exporting tool calls (last 24h)..."
	@$(PSQL) -c "COPY ( \
		SELECT tc.created_at, tc.session_id, tc.message_id, tc.round, \
		       tc.tool_name, tc.input::text as input, \
		       LEFT(tc.output, 500) as output_preview, \
		       tc.latency_ms \
		FROM tool_calls tc \
		WHERE tc.created_at > NOW() - INTERVAL '24 hours' \
		ORDER BY tc.created_at \
	) TO STDOUT WITH CSV HEADER" > reports/tool-calls-$(TIMESTAMP).csv
	@echo "Saved to reports/tool-calls-$(TIMESTAMP).csv"

report-sessions:
	@echo "Exporting session summary..."
	@$(PSQL) -c "COPY ( \
		SELECT cs.id, cs.title, cs.mode_id, cs.created_at, \
		       COUNT(DISTINCT cm.id) as message_count, \
		       COALESCE(SUM(tu.input_tokens), 0) as total_input_tokens, \
		       COALESCE(SUM(tu.output_tokens), 0) as total_output_tokens, \
		       COUNT(DISTINCT tu.id) as llm_rounds \
		FROM chat_sessions cs \
		LEFT JOIN chat_messages cm ON cm.session_id = cs.id \
		LEFT JOIN token_usage tu ON tu.session_id = cs.id \
		GROUP BY cs.id, cs.title, cs.mode_id, cs.created_at \
		ORDER BY cs.created_at DESC \
		LIMIT 50 \
	) TO STDOUT WITH CSV HEADER" > reports/sessions-$(TIMESTAMP).csv
	@echo "Saved to reports/sessions-$(TIMESTAMP).csv"

report-all: report-last-run report-token-usage report-tool-calls report-sessions
	@echo "All reports generated in reports/"
