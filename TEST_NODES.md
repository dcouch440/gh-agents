# Test: Word Frequency Analyzer (File-First Transport)

Three nodes. Copy each description into a board node, wire edges 1→2→3.

## Node 1: Research & PRD

Research and write a PRD for a Python word frequency analyzer CLI. The tool should accept a text file path, count word frequencies, and output a sorted table of the top N words. Research best practices for CLI argument parsing, text processing, and output formatting. Produce a PRD document covering: requirements, technical approach, libraries to use, CLI interface design, and example usage.

## Node 2: Implement

Implement the word frequency analyzer CLI from the PRD in the previous step. Follow the PRD's technical approach. Include: main script, requirements.txt, unit tests, and a sample text file for testing. Install dependencies, run the tests, and verify the CLI works against the sample file.

## Node 3: Run & Compare

Install and run the word frequency analyzer from the previous step against its sample text file. Then create a second, larger text file with a paragraph about artificial intelligence. Run the analyzer against both files and compare results. Report the top 10 words from each.

## What to verify

- Node 1: workforce (researcher + writer) produces PRD
- Node 2: workforce (developer + tester) reads PRD, builds app, runs tests
- Node 3: single agent runs app against real files, captures CLI output
- Files and installed packages persist across all 3 containers via JuiceFS overlay
- Handoff chain threads naturally: PRD → implementation → execution
- Agents use relative paths (no /workspace/ prefix)
- Agents chain commands with && and use heredocs for file creation

---

# Test: Data Pipeline with SQLite

Three nodes, wire 1→2→3. Tests data processing, sqlite3, jq, and cross-step file persistence.

## Node 1: Generate Data

Write a Python script that generates a CSV file of 100 fake sales transactions with columns: date, product, quantity, price, region. Use random data. Run the script and save the CSV.

## Node 2: Load & Query

Create a SQLite database from the CSV. Write SQL queries to find: total revenue by product, top 5 products by quantity sold, and average price by region. Save query results as a JSON report.

## Node 3: Summarize

Read the JSON report from the previous step. Write an executive summary in Markdown with key findings, a recommendation for which region to expand in, and include the raw numbers. Save as final_report.md.

## What to verify

- sqlite3 used for real queries (not Python fakery)
- jq or python for CSV→JSON conversion
- Files persist: CSV → SQLite DB → JSON → Markdown
- Agents use && chaining and heredocs efficiently
- No unnecessary ls or verification calls

---

# Test: Full-Stack Micro App

Four nodes, wire 1→2→3→4. Tests Node.js, multi-file projects, and complex builds.

## Node 1: Design API

Design a REST API for a todo list app. Document endpoints (GET /todos, POST /todos, DELETE /todos/:id), request/response schemas, and error handling. Save as api_spec.md.

## Node 2: Implement Backend

Implement the API from the spec using Node.js and Express. Include package.json, the server file, and in-memory storage. Install dependencies and verify the server starts.

## Node 3: Write Tests

Write integration tests for the API using the test framework of your choice. Run all tests against the server. Save a test results report.

## Node 4: Demo

Start the server, use curl to create 3 todos, list them, delete one, list again. Capture all curl commands and responses in a demo_log.md.

## What to verify

- Node.js + npm used (not Python)
- npm install persists to next steps
- Server starts and responds to real HTTP requests
- curl commands show actual API responses
- 4-step chain with real cross-step dependencies

---

# Test: Parallel Fan-In Research

Three nodes. Wire Node 1→Node 3, Node 2→Node 3. Tests parallel execution and fan-in handoff.

## Node 1: Research Python

Research the current state of Python in 2026 — new features, performance improvements, ecosystem changes. Save findings as python_report.md.

## Node 2: Research Rust

Research the current state of Rust in 2026 — adoption trends, new features, major projects using it. Save findings as rust_report.md.

## Node 3: Compare

Read both reports from the previous steps. Write a comparison analysis: when to choose Python vs Rust, performance tradeoffs, ecosystem maturity, learning curve. Include a recommendation matrix. Save as comparison.md.

## What to verify

- Nodes 1 and 2 execute in parallel (same topological level)
- Node 3 sees both reports via fan-in handoff (multiple <previous_step> blocks)
- Web search used for current 2026 data
- Parallel overlay merge works (both steps write to same workspace)
- Node 3's comparison references specific findings from both reports
