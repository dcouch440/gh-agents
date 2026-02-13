# SEC Filing Analyzer — Protocol Node

## Vision

A new DAG execution mode (`execution_mode = "sec_filing"`) that fetches SEC filings from EDGAR, chunks them by section, runs parallel LLM extraction across sections, and produces a structured diff report comparing the current filing against a prior period. The output is a dense, human-readable summary designed for agents to surface to users — or for downstream nodes to consume as structured JSON.

The core value: nobody reads 200-page 10-Ks. This node reads them for you and tells you **what changed**.

---

## Output Format

The final `StepExecutionEnvelope.data` is a JSON object. When rendered for human consumption (CLI, frontend card, or agent response), it produces:

```
+-- AAPL 10-K FY2025 vs FY2024 ----------------------+
|                                                      |
| FINANCIALS:                                          |
|  Revenue    $394.3B -> $412.1B    (+4.5%)            |
|  Net Income $97.0B  -> $101.2B   (+4.3%)            |
|  Debt       $111.1B -> $98.4B    (-11.4%)           |
|  Cash       $29.9B  -> $34.1B    (+14.0%)           |
|                                                      |
| NEW RISK FACTORS:                                    |
|  - "Regulatory actions in the European Union         |
|     regarding digital markets"                       |
|  - "Supply concentration in advanced semiconductor   |
|     manufacturing"                                   |
|                                                      |
| REMOVED RISK FACTORS:                                |
|  - COVID-related supply chain language               |
|                                                      |
| LEGAL:                                               |
|  Active litigation: 3 new, 2 resolved                |
|  - NEW: DOJ antitrust (App Store)                    |
|  - RESOLVED: Epic Games (settled)                    |
|                                                      |
| SEGMENT SHIFTS:                                      |
|  Services    38% (+4%)                               |
|  iPhone      32% (-3%)                               |
|  Mac          9% (+1%)                               |
|  iPad         7% (-1%)                               |
|  Wearables    7% (-1%)                               |
|                                                      |
| COMPENSATION:                                        |
|  CEO total comp: $63.2M -> $74.6M (+18%)            |
|  New exec: Jane Doe, SVP AI/ML (hired Q2)           |
+------------------------------------------------------+
```

---

## Architecture

### Phased Pipeline (mirrors documenter pattern)

```
Fetch --> Parse --> Extract --> Diff --> Synthesize
```

1. **Fetch** — HTTP calls to SEC EDGAR APIs. No LLM needed. Retrieves the target filing and (optionally) the comparison filing.
2. **Parse** — Chunk the filing HTML/XBRL into logical sections (Risk Factors, MD&A, Financial Statements, Legal Proceedings, Executive Compensation, etc.). Pure Rust, no LLM.
3. **Extract** — Parallel LLM calls per section. Each call receives one section and returns structured JSON (key metrics, notable language, risk items). This is where Haiku shines — structured extraction from predictable formats.
4. **Diff** — If a comparison filing exists, diff the extracted data section by section. Pure Rust — compute deltas, flag new/removed items, calculate percentage changes.
5. **Synthesize** — Single LLM call to produce the final summary. Receives all section diffs and produces the human-readable report plus the structured JSON envelope.

### Module Structure

```
src/server/hub/dag/
  sec_filing/
    mod.rs        -- SecFilingExecutor, phased pipeline orchestration
    fetch.rs      -- EDGAR API client, filing retrieval
    parse.rs      -- HTML/XBRL sectioning, chunk extraction
    extract.rs    -- Per-section LLM extraction logic
    diff.rs       -- Section-level diffing, delta computation
    types.rs      -- FilingSection, ExtractedData, FilingDiff, etc.
    prompts.rs    -- System/user prompts for extract and synthesize phases
    tests.rs      -- Unit and integration tests

config/protocols/sec_filing/
    config.yaml   -- Agent configs (extractor, synthesizer)
    extractor/
      system.md   -- System prompt for section extraction
      prompt.md   -- User prompt template
      response.json -- Expected JSON schema
    synthesizer/
      system.md   -- System prompt for final synthesis
      prompt.md   -- User prompt template
```

### Strategy Layer

```
src/server/hub/strategies/
  sec_filing/
    mod.rs        -- SecFilingExtractorStrategy, SecFilingSynthesizerStrategy
    tests.rs
```

Two strategies following the `ExecutionStrategy` trait:

- **`SecFilingExtractorStrategy`** — Single-shot, no tools, low temperature (0.1). Receives one section, returns structured JSON. Uses Haiku-tier model for cost efficiency.
- **`SecFilingSynthesizerStrategy`** — Single-shot, no tools, moderate temperature (0.3). Receives all section extractions + diffs, produces the final report. Uses Sonnet-tier model for quality.

### Database

Migration: `0029_sec_filing_config.sql`

```sql
CREATE TABLE sec_filing_configs (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id         uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    ticker          text NOT NULL,
    filing_type     text NOT NULL DEFAULT '10-K',  -- 10-K, 10-Q, 8-K
    compare_period  text,                           -- 'previous', 'yoy', or specific accession number
    sections        text[] DEFAULT '{}',            -- empty = all sections
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE(step_id)
);
```

New row type in `src/db/mod.rs`:

```rust
pub struct SecFilingConfigRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub ticker: String,
    pub filing_type: String,
    pub compare_period: Option<String>,
    pub sections: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

DB trait additions in `src/db/traits/mod.rs`:

```rust
async fn get_sec_filing_config(&self, step_id: Uuid) -> Result<Option<SecFilingConfigRow>>;
async fn upsert_sec_filing_config(&self, step_id: Uuid, ticker: &str, filing_type: &str, compare_period: Option<&str>, sections: &[String]) -> Result<SecFilingConfigRow>;
```

### EDGAR Client

Lives in `src/server/hub/dag/sec_filing/fetch.rs`. Pure HTTP — no auth required.

Key endpoints:
- **Full-text search**: `https://efts.sec.gov/LATEST/search-index?q=...&forms=10-K&entity=AAPL`
- **Company filings**: `https://data.sec.gov/submissions/CIK{cik}.json`
- **Filing document**: `https://www.sec.gov/Archives/edgar/data/{cik}/{accession}/{filename}`
- **XBRL facts** (structured financials): `https://data.sec.gov/api/xbrl/companyfacts/CIK{cik}.json`

The EDGAR API requires a `User-Agent` header with contact info (SEC policy). Use a configurable value from env or app config.

### Section Parser

The 10-K/10-Q follows Regulation S-K. Standard sections to extract:

| Section | Reg S-K Item | Content |
|---------|-------------|---------|
| Risk Factors | Item 1A | What could go wrong |
| MD&A | Item 7 | Management's narrative on results |
| Financial Statements | Item 8 | Balance sheet, income, cash flow |
| Legal Proceedings | Item 3 | Active litigation |
| Executive Compensation | Item 11 | Pay tables, equity grants |
| Business Overview | Item 1 | What the company does |
| Properties | Item 2 | Real estate and facilities |

The parser uses heading detection (HTML `<b>`, `<font>`, or XBRL tags) to split the document. For structured financials, prefer XBRL over HTML parsing — the data is already machine-readable.

### Integration into DAG

In `src/server/hub/dag/mod.rs`, add the dispatch branch:

```rust
if step.execution_mode == "sec_filing" {
    let step_result = execute_sec_filing_step(
        engine, state, ctx, step, steps, edges,
        var_outputs, completed, completed_envelopes,
        &port_meta, total_input_tokens, total_output_tokens,
        total_cost_usd, cancel.as_ref(),
    ).await?;
    continue;
}
```

### Designer Input

In `src/server/hub/dag/designer_input/`, add `sec_filing.rs` so the node assistant can configure this node type through conversation. The assistant recognizes intents like "analyze Apple's latest 10-K" or "compare Tesla's annual filings" and creates the `sec_filing_configs` row.

---

## Structured Output Schema

The `StepExecutionEnvelope.data` JSON:

```json
{
  "ticker": "AAPL",
  "filing_type": "10-K",
  "period": "FY2025",
  "compare_period": "FY2024",
  "sections": {
    "financials": {
      "revenue": { "current": 412100000000, "prior": 394300000000, "delta_pct": 4.5 },
      "net_income": { "current": 101200000000, "prior": 97000000000, "delta_pct": 4.3 },
      "total_debt": { "current": 98400000000, "prior": 111100000000, "delta_pct": -11.4 },
      "cash": { "current": 34100000000, "prior": 29900000000, "delta_pct": 14.0 }
    },
    "risk_factors": {
      "new": ["Regulatory actions in the EU regarding digital markets", "..."],
      "removed": ["COVID-related supply chain language"],
      "unchanged_count": 12
    },
    "legal": {
      "new_cases": [{ "name": "DOJ antitrust", "subject": "App Store" }],
      "resolved_cases": [{ "name": "Epic Games", "resolution": "settled" }]
    },
    "segments": {
      "Services": { "current_pct": 38, "prior_pct": 34, "delta": 4 },
      "iPhone": { "current_pct": 32, "prior_pct": 35, "delta": -3 }
    },
    "compensation": {
      "ceo_total": { "current": 74600000, "prior": 63200000, "delta_pct": 18.0 },
      "new_executives": [{ "name": "Jane Doe", "title": "SVP AI/ML", "hired": "Q2" }]
    },
    "mda_summary": "Management highlighted continued growth in Services..."
  },
  "summary": "The rendered text report (box-drawn format)"
}
```

---

## Model Selection and Cost

| Phase | Model | Rationale | Est. Cost per Filing |
|-------|-------|-----------|---------------------|
| Extract (per section, ~7 sections) | Haiku | Structured extraction, predictable format | ~$0.002 |
| Synthesize | Sonnet | Narrative quality, cross-referencing | ~$0.02 |
| **Total** | | | **~$0.025** |

At this cost, you can analyze all S&P 500 companies quarterly for ~$12.50.

---

## Protocol Config

`config/protocols/sec_filing/config.yaml`:

```yaml
agents:
  extractor:
    model_id: claude-haiku-4-5-20251001
    max_tokens: 4096
    temperature: 0.1
    max_rounds: 1
    context_budget: 100000
  synthesizer:
    model_id: claude-sonnet-4-20250514
    max_tokens: 8192
    temperature: 0.3
    max_rounds: 1
    context_budget: 200000
```

---

## Testing

### Unit Tests (`sec_filing/tests.rs`)

- **Parse**: Feed sample 10-K HTML, verify section boundaries are detected correctly
- **Diff**: Two extracted section JSONs in, verify deltas computed correctly
- **Extract prompt**: Verify prompt template renders with section content
- **Config DB**: CRUD for `sec_filing_configs` table

### Integration Tests

- **Full pipeline** with mocked EDGAR responses and mocked LLM (use existing `MockProvider` pattern)
- **Missing comparison filing** — should produce current-period-only report, no diff
- **Partial section failure** — one section extraction fails, rest succeed, report marks it as `"extraction_failed"`
- **XBRL fallback** — when XBRL is available, financials come from structured data, not LLM extraction

---

## Edge Cases

- **Filing not found**: Return `ExecutionStatus::Error` with clear message ("No 10-K found for ticker FOO")
- **Rate limiting**: EDGAR rate-limits to 10 req/sec. The fetch module should respect this with a semaphore or token bucket.
- **Large filings**: Some 10-Ks exceed 500 pages. The parser should cap section size at the `context_budget` and truncate with a note.
- **Foreign filers**: 20-F instead of 10-K. Support as a `filing_type` variant.
- **XBRL availability**: Not all filings have inline XBRL. Fall back to HTML parsing for financials when XBRL is absent.

---

## Dependencies

- `reqwest` (already in Cargo.toml) for EDGAR HTTP calls
- `scraper` or `select` crate for HTML section parsing (evaluate which is lighter)
- No new external API keys required — EDGAR is fully public

---

## Downstream Composability

This node's `StepExecutionEnvelope` output is fully port-compatible. Downstream nodes can:

- **Belief Capture** node: Extract beliefs from the filing analysis ("Apple is reducing debt aggressively")
- **Room/Meeting** node: Discuss the filing results with multiple AI participants
- **Another SEC Filing** node: Chain multiple tickers for comparative analysis
- **Task Force** node: Act on findings ("Draft an investment memo based on this analysis")

The structured JSON in `data` is extractable via `json_path` in port resolution, so any field (e.g., `$.sections.risk_factors.new`) can be piped to downstream steps.
