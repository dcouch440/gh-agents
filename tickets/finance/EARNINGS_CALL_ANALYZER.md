# Earnings Call Transcript Analyzer — Protocol Node

## Vision

A new DAG execution mode (`execution_mode = "earnings_call"`) that fetches earnings call transcripts, splits them into prepared remarks and Q&A, runs parallel LLM analysis passes (tone scoring, topic frequency, dodge detection, guidance language), and produces a structured diff report comparing against the prior quarter's call. The output surfaces **what changed in how management talks** — not the numbers (which are priced in instantly), but the language shifts that humans miss.

---

## Output Format

The final `StepExecutionEnvelope.data` is a JSON object. When rendered for human consumption:

```
+-- AAPL Q1 2025 vs Q4 2024 -------------------------+
|                                                      |
| TONE SHIFT: Cautious <-- Confident  (-2)             |
|                                                      |
| NEW LANGUAGE:                                        |
|  - "disciplined capital allocation" (3x)             |
|  - "normalizing demand" (2x)                         |
|                                                      |
| DROPPED LANGUAGE:                                    |
|  - "record quarter" (was 4x, now 0)                  |
|  - "China growth" (was 6x, now 0)                    |
|                                                      |
| TOPIC FREQUENCY:                                     |
|  AI/ML          12x (+5)                             |
|  Services        8x (+2)                             |
|  Hardware        4x (-6)                             |
|  China           0x (-6)                             |
|                                                      |
| Q&A RED FLAGS:                                       |
|  - Margin question deflected (2x)                    |
|  - Inventory question --> vague answer               |
|                                                      |
| GUIDANCE LANGUAGE:                                   |
|  Q4: "We expect strong growth"                       |
|  Q1: "We anticipate steady performance"              |
|  Delta: Downgrade (strong->steady, expect->          |
|         anticipate)                                  |
+------------------------------------------------------+
```

---

## Architecture

### Phased Pipeline

```
Fetch --> Segment --> Analyze --> Diff --> Synthesize
```

1. **Fetch** — Retrieve the earnings call transcript. Multiple source adapters (Motley Fool scraper, direct IR page, or user-provided text). No LLM needed.
2. **Segment** — Split transcript into structured sections: opening/prepared remarks per speaker, Q&A pairs (analyst question + management response). Pure Rust text processing.
3. **Analyze** — Parallel LLM calls across multiple analysis dimensions. Each dimension is an independent extraction pass over the segmented transcript:
   - **Tone Scoring** — Overall sentiment and confidence level on a -5 to +5 scale
   - **Topic Frequency** — Count mentions of key topics/themes with context
   - **Language Tracking** — Notable phrases, corporate euphemisms, new/dropped terminology
   - **Q&A Quality** — For each analyst question: was it answered directly, deflected, or pivoted away from?
   - **Guidance Extraction** — Forward-looking statements with confidence qualifiers
4. **Diff** — If a prior quarter's analysis exists, compute deltas across all dimensions. Pure Rust.
5. **Synthesize** — Single LLM call producing the final report from all analysis passes + diffs.

### Module Structure

```
src/server/hub/dag/
  earnings_call/
    mod.rs          -- EarningsCallExecutor, phased pipeline orchestration
    fetch.rs        -- Transcript retrieval (multiple source adapters)
    segment.rs      -- Speaker identification, section splitting, Q&A pairing
    analyze.rs      -- Per-dimension LLM analysis orchestration
    diff.rs         -- Quarter-over-quarter delta computation
    types.rs        -- Transcript, Speaker, QAPair, ToneScore, TopicCount, etc.
    prompts.rs      -- System/user prompts for each analysis dimension
    tests.rs        -- Unit and integration tests

config/protocols/earnings_call/
    config.yaml     -- Agent configs (tone_scorer, topic_counter, qa_analyzer, etc.)
    tone_scorer/
      system.md     -- System prompt for tone analysis
      prompt.md     -- User prompt template
      response.json -- Expected JSON schema
    topic_counter/
      system.md
      prompt.md
      response.json
    qa_analyzer/
      system.md
      prompt.md
      response.json
    guidance_extractor/
      system.md
      prompt.md
      response.json
    synthesizer/
      system.md
      prompt.md
```

### Strategy Layer

```
src/server/hub/strategies/
  earnings_call/
    mod.rs          -- Strategy impls for each analysis dimension
    tests.rs
```

Five strategies implementing `ExecutionStrategy`:

| Strategy | Model | Temp | Rounds | Purpose |
|----------|-------|------|--------|---------|
| `ToneScorerStrategy` | Haiku | 0.1 | 1 | Sentiment + confidence scoring |
| `TopicCounterStrategy` | Haiku | 0.1 | 1 | Theme frequency extraction |
| `LanguageTrackerStrategy` | Haiku | 0.1 | 1 | Notable phrase detection |
| `QAAnalyzerStrategy` | Haiku | 0.2 | 1 | Question-response quality assessment |
| `GuidanceExtractorStrategy` | Haiku | 0.1 | 1 | Forward-looking statement extraction |
| `EarningsCallSynthesizerStrategy` | Sonnet | 0.3 | 1 | Final report generation |

All analysis strategies are single-shot, no tools. The synthesizer receives all analysis outputs and produces the final report.

### Database

Migration: `0030_earnings_call_config.sql`

```sql
CREATE TABLE earnings_call_configs (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id         uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    ticker          text NOT NULL,
    quarter         text,                           -- 'Q1 2025', null = latest
    compare_quarter text,                           -- 'previous', specific quarter, or null
    source          text NOT NULL DEFAULT 'auto',   -- 'motley_fool', 'manual', 'auto'
    transcript_text text,                           -- for manual/pasted transcripts
    analysis_dimensions text[] DEFAULT '{}',        -- empty = all dimensions
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE(step_id)
);

-- Cache analyzed results for quarter-over-quarter diffing
CREATE TABLE earnings_call_analyses (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    ticker          text NOT NULL,
    quarter         text NOT NULL,
    analysis_data   jsonb NOT NULL,                 -- Full analysis output
    source_url      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE(ticker, quarter)
);
```

Row types in `src/db/mod.rs`:

```rust
pub struct EarningsCallConfigRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub ticker: String,
    pub quarter: Option<String>,
    pub compare_quarter: Option<String>,
    pub source: String,
    pub transcript_text: Option<String>,
    pub analysis_dimensions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct EarningsCallAnalysisRow {
    pub id: Uuid,
    pub ticker: String,
    pub quarter: String,
    pub analysis_data: JsonValue,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

DB trait additions:

```rust
async fn get_earnings_call_config(&self, step_id: Uuid) -> Result<Option<EarningsCallConfigRow>>;
async fn upsert_earnings_call_config(&self, step_id: Uuid, ticker: &str, quarter: Option<&str>, compare_quarter: Option<&str>, source: &str, transcript_text: Option<&str>, dimensions: &[String]) -> Result<EarningsCallConfigRow>;
async fn get_earnings_call_analysis(&self, ticker: &str, quarter: &str) -> Result<Option<EarningsCallAnalysisRow>>;
async fn upsert_earnings_call_analysis(&self, ticker: &str, quarter: &str, analysis_data: &JsonValue, source_url: Option<&str>) -> Result<EarningsCallAnalysisRow>;
```

The `earnings_call_analyses` table is the key differentiator from the SEC filing tool. By caching each quarter's analysis, subsequent runs only need to analyze the new quarter and diff against stored data — no re-processing of prior transcripts.

### Transcript Sources

#### Motley Fool (Primary - Free)

Transcripts are publicly available at predictable URLs. The fetch module scrapes them:
- Listing page: search for `{ticker} earnings call transcript`
- Transcript page: structured HTML with speaker names and timestamps
- No auth required, no API key

#### Manual Input

Users can paste transcript text directly into the `transcript_text` field. This handles:
- Proprietary transcripts from paid services
- Transcripts from IR pages with non-standard formatting
- YouTube auto-captions (cleaned up)

#### Auto Mode

Tries Motley Fool first. Falls back to prompting the user for manual input if not found.

### Transcript Segmentation

The segmenter (`segment.rs`) produces:

```rust
pub struct SegmentedTranscript {
    pub ticker: String,
    pub quarter: String,
    pub date: NaiveDate,
    pub prepared_remarks: Vec<SpeakerBlock>,
    pub qa_pairs: Vec<QAPair>,
    pub participants: Vec<Participant>,
}

pub struct SpeakerBlock {
    pub speaker: String,
    pub title: Option<String>,    // "CEO", "CFO", etc.
    pub content: String,
}

pub struct QAPair {
    pub analyst: String,
    pub firm: Option<String>,     // "Goldman Sachs", etc.
    pub question: String,
    pub respondent: String,
    pub response: String,
}

pub struct Participant {
    pub name: String,
    pub title: Option<String>,
    pub role: ParticipantRole,    // Management | Analyst
}
```

Speaker identification uses pattern matching on common transcript formats:
- `"Tim Cook -- Chief Executive Officer"` (Motley Fool style)
- `"Tim Cook - CEO"` (SeekingAlpha style)
- `"Operator"` (always the moderator)

### Integration into DAG

In `src/server/hub/dag/mod.rs`:

```rust
if step.execution_mode == "earnings_call" {
    let step_result = execute_earnings_call_step(
        engine, state, ctx, step, steps, edges,
        var_outputs, completed, completed_envelopes,
        &port_meta, total_input_tokens, total_output_tokens,
        total_cost_usd, cancel.as_ref(),
    ).await?;
    continue;
}
```

### Designer Input

In `src/server/hub/dag/designer_input/earnings_call.rs`. The node assistant recognizes intents like:
- "Analyze Apple's latest earnings call"
- "Compare Tesla's Q3 and Q4 calls"
- "What changed in NVIDIA's tone last quarter?"

And creates the `earnings_call_configs` row with appropriate settings.

---

## Structured Output Schema

The `StepExecutionEnvelope.data` JSON:

```json
{
  "ticker": "AAPL",
  "quarter": "Q1 2025",
  "compare_quarter": "Q4 2024",
  "call_date": "2025-01-30",
  "participants": {
    "management": ["Tim Cook (CEO)", "Luca Maestri (CFO)"],
    "analysts": ["Toni Sacconaghi (Bernstein)", "Amit Daryanani (Evercore)"]
  },
  "tone": {
    "current": { "score": 2, "label": "Cautious" },
    "prior": { "score": 4, "label": "Confident" },
    "delta": -2,
    "evidence": [
      "Shifted from 'strong demand' to 'healthy demand'",
      "More hedging language ('we believe', 'we expect') vs prior quarter's definitive statements"
    ]
  },
  "topics": {
    "AI/ML": { "current": 12, "prior": 7, "delta": 5 },
    "Services": { "current": 8, "prior": 6, "delta": 2 },
    "Hardware": { "current": 4, "prior": 10, "delta": -6 },
    "China": { "current": 0, "prior": 6, "delta": -6 }
  },
  "language": {
    "new_phrases": [
      { "phrase": "disciplined capital allocation", "count": 3, "context": "CFO used in response to buyback questions" },
      { "phrase": "normalizing demand", "count": 2, "context": "CEO discussing iPhone segment" }
    ],
    "dropped_phrases": [
      { "phrase": "record quarter", "prior_count": 4 },
      { "phrase": "China growth", "prior_count": 6 }
    ]
  },
  "qa_quality": {
    "total_questions": 14,
    "direct_answers": 9,
    "deflections": 3,
    "pivots": 2,
    "red_flags": [
      {
        "analyst": "Toni Sacconaghi",
        "topic": "Gross margins",
        "assessment": "deflected",
        "detail": "Asked about margin pressure from AI infrastructure spend. CEO pivoted to revenue growth narrative without addressing margin question."
      }
    ]
  },
  "guidance": {
    "current": [
      { "statement": "We anticipate steady performance", "confidence": "moderate", "qualifier": "anticipate" }
    ],
    "prior": [
      { "statement": "We expect strong growth", "confidence": "high", "qualifier": "expect" }
    ],
    "assessment": "Downgrade: 'strong' -> 'steady', 'expect' -> 'anticipate'"
  },
  "summary": "The rendered text report"
}
```

---

## Model Selection and Cost

| Phase | Parallel Calls | Model | Est. Cost |
|-------|---------------|-------|-----------|
| Tone Scoring | 1 | Haiku | ~$0.0005 |
| Topic Frequency | 1 | Haiku | ~$0.0005 |
| Language Tracking | 1 | Haiku | ~$0.0005 |
| Q&A Analysis | 1 | Haiku | ~$0.001 |
| Guidance Extraction | 1 | Haiku | ~$0.0005 |
| Synthesize | 1 | Sonnet | ~$0.02 |
| **Total** | | | **~$0.023** |

The five analysis passes run in parallel via `tokio::task::JoinSet` (same pattern as for-each iteration). Total wall-clock time is dominated by the slowest single pass, not the sum.

---

## Protocol Config

`config/protocols/earnings_call/config.yaml`:

```yaml
agents:
  tone_scorer:
    model_id: claude-haiku-4-5-20251001
    max_tokens: 2048
    temperature: 0.1
    max_rounds: 1
    context_budget: 100000
  topic_counter:
    model_id: claude-haiku-4-5-20251001
    max_tokens: 2048
    temperature: 0.1
    max_rounds: 1
    context_budget: 100000
  language_tracker:
    model_id: claude-haiku-4-5-20251001
    max_tokens: 2048
    temperature: 0.1
    max_rounds: 1
    context_budget: 100000
  qa_analyzer:
    model_id: claude-haiku-4-5-20251001
    max_tokens: 4096
    temperature: 0.2
    max_rounds: 1
    context_budget: 100000
  guidance_extractor:
    model_id: claude-haiku-4-5-20251001
    max_tokens: 2048
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

### Unit Tests (`earnings_call/tests.rs`)

- **Segment**: Feed sample transcript text, verify speakers are identified, Q&A pairs are extracted correctly, titles are parsed
- **Diff**: Two quarter analysis JSONs in, verify topic deltas, tone shifts, new/dropped phrases computed correctly
- **Prompt rendering**: Verify each analysis dimension's prompt template renders with transcript content
- **Config DB**: CRUD for `earnings_call_configs` and `earnings_call_analyses` tables
- **Speaker parsing**: Various transcript formats (Motley Fool, SeekingAlpha, raw) produce correct `Participant` structs

### Integration Tests

- **Full pipeline** with mocked transcript source and mocked LLM
- **No comparison quarter** — first-ever analysis for a ticker, produces current-only report
- **Cached comparison** — prior quarter exists in `earnings_call_analyses`, only current quarter is processed
- **Manual transcript** — `transcript_text` provided, skip fetch entirely
- **Partial dimension failure** — one analysis pass fails, rest succeed, report marks it accordingly
- **Multiple speakers** — CFO, CEO, COO all present, verify speaker blocks are attributed correctly

---

## Edge Cases

- **Transcript not found**: Return `ExecutionStatus::Error` with message suggesting manual input
- **Non-English transcripts**: Out of scope for v1. Flag and skip with a clear message.
- **Operator sections**: The "Operator" speaker introduces Q&A and provides logistical info. The segmenter should identify and exclude operator text from analysis.
- **Multiple questions per analyst**: Some analysts ask 2-3 questions in sequence. The segmenter should split these into separate `QAPair` entries.
- **Company name variations**: "Apple Inc." vs "AAPL" vs "Apple" — the fetch module should handle ticker-to-company-name resolution.
- **Transcript length**: Most calls are 8,000-15,000 words. Within Haiku's context window easily. No chunking needed for analysis passes — each gets the full transcript.

---

## Downstream Composability

This node's output is fully port-compatible for downstream consumption:

- **SEC Filing Analyzer** node: Cross-reference earnings call tone with 10-K risk factors ("management is cautious on calls but risk factors haven't changed — disconnect?")
- **Belief Capture** node: Extract investment beliefs from the analysis ("AAPL is pivoting from hardware to services")
- **Room/Meeting** node: Discuss earnings call findings with multiple AI analysts
- **For-Each** node: Iterate over multiple tickers, producing a comparative dashboard
- **Task Force** node: "Draft a client note based on this earnings call analysis"

The `topics` and `qa_quality.red_flags` fields are particularly valuable for downstream nodes — they provide structured, extractable signals via `json_path` port resolution.

---

## Relationship to SEC Filing Analyzer

These two tools are designed to work together. The SEC filing gives you the **facts** (numbers, risks, legal). The earnings call gives you the **narrative** (how management frames those facts). The delta between the two is where the real insight lives.

A typical workflow DAG:

```
[SEC Filing: AAPL 10-K] ----+
                              +--> [Belief Capture] --> [Room: Investment Committee]
[Earnings Call: AAPL Q4] ---+
```

The Belief Capture node receives both outputs, extracts cross-referenced beliefs, and the Room node debates them.
