# Research: On-Demand Node Text Editing via Semantic Search

## Problem Statement

When a user talks to an assistant and requests changes ("make it about cats instead of dogs"), the system needs to:
1. Find which node(s) across the workflow are relevant to the change
2. Find which lines within the relevant node(s) need editing
3. Apply targeted, surgical edits — not wholesale replacement

The canvas text must stay current as the source of truth. Agents keep it in sync by making search/replace edits, like Claude Code editing files.

## Architecture Decision

```
User: "I want cats instead of dogs"
         |
         v
┌────────────────────────────────────────────┐
│  Assistant / Coordinator                   │
│                                            │
│  1. Cosine search across all node texts    │
│     → finds relevant node(s)              │
│  2. Dispatches to node agent with:         │
│     - instruction                          │
│     - full node text                       │
│     - node's current DB config             │
└─────────────────┬──────────────────────────┘
                  |
                  v
┌────────────────────────────────────────────┐
│  Node Agent                                │
│                                            │
│  Receives full text of its own node        │
│  + instruction from assistant              │
│  Outputs search/replace edits as JSON      │
│                                            │
│  {                                         │
│    "edits": [{                             │
│      "search": "story about dogs",         │
│      "replace": "story about cats"         │
│    }]                                      │
│  }                                         │
└─────────────────┬──────────────────────────┘
                  |
                  v
┌────────────────────────────────────────────┐
│  Apply Engine                              │
│                                            │
│  For each edit:                            │
│    1. Verify "search" exists in node text  │
│    2. Verify "search" is unique            │
│    3. String replace                       │
│    4. Reject if verification fails         │
└────────────────────────────────────────────┘
```

### Two-Layer Search

| Layer | What it finds | Who does it | How |
|-------|--------------|-------------|-----|
| **Node selection** | Which node(s) are relevant | Assistant/coordinator | Cosine similarity across whole-node embeddings |
| **Line selection** | Which lines to edit | The LLM agent itself | Reads full node text, outputs search/replace |

Cosine search = routing decision (which node to dispatch to).
LLM = editing decision (what to change within the node).

No embedding search is used within a node — the documents are too small (5-30 lines, ~50-300 tokens) to benefit from retrieval. The entire node text fits trivially in a single LLM context window.

---

## Part 1: Embedding APIs for Node-Level Search

### Recommended: OpenAI `text-embedding-3-small`

| Property | Value |
|----------|-------|
| Dimensions | 1536 default, configurable (256, 512, 1024, 1536) via Matryoshka |
| Max input | 8,192 tokens per string |
| Batch limit | 300,000 tokens total / 2,048 inputs per request |
| Price | $0.02 / 1M tokens |
| Latency | P50: ~200-400ms, P90: ~500ms |
| Normalization | Pre-normalized to unit length (cosine = dot product) |

At our scale (20 nodes x ~100 tokens = 2,000 tokens per request), cost is ~$0.00004 per query. Effectively free. Everything fits in a single batch API call.

#### Request/Response

```bash
POST https://api.openai.com/v1/embeddings
Authorization: Bearer $OPENAI_API_KEY

{
  "model": "text-embedding-3-small",
  "input": [
    "I want to create a story about dogs...",
    "Research competitors and summarize findings",
    "Generate executive summary"
  ],
  "dimensions": 512
}
```

```json
{
  "object": "list",
  "data": [
    { "object": "embedding", "embedding": [0.0023, -0.0093, ...], "index": 0 },
    { "object": "embedding", "embedding": [-0.0076, 0.0201, ...], "index": 1 },
    { "object": "embedding", "embedding": [0.0112, -0.0032, ...], "index": 2 }
  ],
  "model": "text-embedding-3-small",
  "usage": { "prompt_tokens": 28, "total_tokens": 28 }
}
```

### Alternative Providers

| Provider | Model | Dims | Price / 1M tokens | Max Input | Notes |
|----------|-------|------|-------------------|-----------|-------|
| **OpenAI** | text-embedding-3-small | 512 (configurable) | $0.02 | 8K tokens | Best quality/price. Pre-normalized. |
| **OpenAI** | text-embedding-3-large | 3072 (configurable) | $0.13 | 8K tokens | Higher quality, 6.5x cost |
| **Voyage AI** | voyage-3.5-lite | 1024 (configurable) | $0.02 | 32K tokens | 200M free tokens. OpenAI-compatible API. |
| **Voyage AI** | voyage-3.5 | 1024 (configurable) | $0.06 | 32K tokens | Top MTEB scores. |
| **Google** | text-embedding-004 | 768 | **Free** | 2K tokens | Completely free tier. Decent quality. |
| **Google** | gemini-embedding-001 | 3072 (configurable) | $0.15 (free tier available) | 8K tokens | Competitive quality. |
| **Cohere** | embed-english-v3.0 | 1024 | $0.10 | 512 tokens | 512-token limit is restrictive. |
| **xAI** | (undocumented) | ? | ? | ? | Endpoint exists but sparse docs. Not recommended. |

### Local Alternative: `fastembed-rs`

For zero-network-dependency embedding:

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

let model = TextEmbedding::try_new(InitOptions {
    model_name: EmbeddingModel::BGESmallENV15,
    ..Default::default()
})?;

let embeddings = model.embed(vec!["text to embed"], None)?;
// Vec<Vec<f32>>, 384 dimensions
```

| Factor | Local (fastembed) | API (OpenAI) |
|--------|------------------|--------------|
| Latency | 5-50ms for 50 texts | 200-500ms |
| Cold start | 200ms-2s model load | None |
| Quality (MTEB) | ~63-66 | ~68-72 |
| Cost | 0 per query, ~50-200MB RAM | $0.00004 per query |
| Dependencies | ONNX Runtime (~20MB) + model (~25MB) | Just reqwest |

**Recommendation**: Use OpenAI for board operations where 300-500ms is fine and quality matters. Consider local embeddings later if latency becomes critical.

---

## Part 2: Cosine Similarity in Rust

### Implementation (zero dependencies)

```rust
/// Cosine similarity between two f32 vectors.
/// Returns 0.0 if either vector has zero magnitude.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let (mut dot, mut norm_a, mut norm_b) = (0.0_f32, 0.0_f32, 0.0_f32);

    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom < f32::EPSILON {
        0.0
    } else {
        (dot / denom).clamp(-1.0, 1.0)
    }
}
```

**Note**: OpenAI embeddings are pre-normalized to unit length. For pre-normalized vectors, cosine similarity = dot product:

```rust
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}
```

### Top-K Search

```rust
pub struct ScoredNode {
    pub index: usize,
    pub score: f32,
}

pub fn top_k(query: &[f32], corpus: &[&[f32]], k: usize) -> Vec<ScoredNode> {
    let mut scores: Vec<ScoredNode> = corpus
        .iter()
        .enumerate()
        .map(|(i, doc)| ScoredNode {
            index: i,
            score: dot_product(query, doc),
        })
        .collect();

    scores.select_nth_unstable_by(
        k.min(scores.len()).saturating_sub(1),
        |a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal),
    );

    scores.truncate(k);
    scores.sort_unstable_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });
    scores
}
```

### Performance at Our Scale

- 20 nodes x 512-dim embeddings = 40KB memory
- 20 cosine similarity computations: ~1-2 microseconds
- SIMD not needed — bottleneck is always the embedding API call, not the math
- No external crates needed. If profiling ever shows this matters, `simsimd` crate is the upgrade path.

### Text Preprocessing Before Embedding

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static MULTI_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

pub fn prepare_for_embedding(text: &str) -> String {
    let no_newlines = text.replace('\n', " ");
    MULTI_WS.replace_all(&no_newlines, " ").trim().to_string()
}
```

Key rules:
- Replace newlines with spaces (OpenAI docs note inferior results with newlines)
- Collapse whitespace
- Do NOT lowercase (models use case as signal)
- Do NOT remove stop words (transformers use them for context)

### Calling OpenAI from Rust (reqwest)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbeddingRequest {
    model: &'static str,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<u32>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

async fn embed_texts(
    client: &Client,
    api_key: &str,
    texts: Vec<String>,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let resp = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&EmbeddingRequest {
            model: "text-embedding-3-small",
            input: texts,
            dimensions: Some(512),
        })
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .json::<EmbeddingResponse>()
        .await?;

    let mut data = resp.data;
    data.sort_by_key(|d| d.index);
    Ok(data.into_iter().map(|d| d.embedding).collect())
}
```

---

## Part 3: Edit Format — Search/Replace

### Why Search/Replace Wins

Research from Aider, Claude Code, OpenAI's apply_patch.py, and the "Code Surgery" analysis (Hertwig, 2025) converge on the same conclusion:

| Format | Pros | Cons |
|--------|------|------|
| **Whole file** | Simple, works with weak models | Expensive in tokens, high latency |
| **Line-number diffs** | Compact | LLMs miscount lines regularly |
| **Unified diff** | Token-efficient | Fragile (line numbers, context lines) |
| **Search/replace** | LLMs excel at reproducing text they just read. Verifiable. No line numbers. | Requires the search text to exist and be unique |

**Key finding**: "Successful formats often avoid line numbers and clearly provide both the code to be replaced and its replacement, using distinct delimiters." — OpenAI GPT-4.1 Prompt Cookbook

### Structured JSON Output

The node agent should output structured JSON edits:

```json
{
  "node_id": "step-abc-123",
  "edits": [
    {
      "search": "I want to create a story about dogs...\nThis is why dogs are great",
      "replace": "I want to create a story about cats...\nThis is why cats are great",
      "reasoning": "User requested changing subject from dogs to cats"
    }
  ]
}
```

### Apply Engine (Rust)

```rust
fn apply_edit(text: &str, search: &str, replace: &str) -> Result<String, EditError> {
    // 1. Verify search text exists
    if !text.contains(search) {
        return Err(EditError::SearchNotFound(search.to_string()));
    }

    // 2. Verify search text is unique
    let count = text.matches(search).count();
    if count > 1 {
        return Err(EditError::AmbiguousMatch { search: search.to_string(), count });
    }

    // 3. Apply replacement
    Ok(text.replacen(search, replace, 1))
}
```

### Prompt Template for Node Agent

```xml
<system>
You edit workflow node text. You receive the full text of a node and an
instruction describing what to change. Output a JSON object with an "edits"
array containing search/replace pairs.

Rules:
- The "search" value must be an exact substring of the node text
- The "search" value must be unique within the node text
- Include enough surrounding context in "search" to be unambiguous
- Do not use line numbers
- If no edit is needed, return {"edits": []}
</system>

<user>
Instruction: {dispatch_instruction}

Node text:
```
{full_node_text}
```
</user>
```

---

## Part 4: When to Scale

### Current Scale (No Retrieval Needed)

| Metric | Value | Implication |
|--------|-------|-------------|
| Nodes per workflow | 5-20 | All fit in one embedding batch + one LLM context |
| Lines per node | 5-30 | Full text fits in a single embedding vector's input |
| Total tokens (all nodes) | ~2,000-6,000 | Trivially fits in LLM context |
| Cost per edit query | ~$0.005-0.02 | Negligible |

At this scale, embed all nodes in one batch call, cosine search to find relevant ones, send full text to the agent. No chunking, no within-node retrieval, no vector database.

### When to Add Infrastructure

| Trigger | What to Add |
|---------|-------------|
| **50+ nodes** per workflow | BM25 or embedding pre-filter to top 5-10 nodes |
| **100+ lines** per node | Within-node paragraph-level chunking |
| **Sub-100ms latency** requirement | Local embeddings via `fastembed-rs` |
| **Frequent repeated queries** on same nodes | Cache node embeddings (invalidate on edit) |

### Anti-Pattern to Avoid

> "If your data source already has small, complete pieces of information like FAQs, product descriptions, or social media posts, you usually do not need to chunk them." — Pinecone Chunking Guide

Node texts are exactly this: small, complete pieces of information. The correct retrieval strategy at this scale is "include everything."

---

## Sources

### Embedding APIs
- [OpenAI Embeddings Guide](https://developers.openai.com/api/docs/guides/embeddings/)
- [OpenAI New Embedding Models (Matryoshka)](https://openai.com/index/new-embedding-models-and-api-updates/)
- [Voyage AI Embeddings API Reference](https://docs.voyageai.com/reference/embeddings-api)
- [Google Gemini Embedding](https://developers.googleblog.com/gemini-embedding-available-gemini-api/)
- [Nixiesearch: Benchmarking Embedding API Latency](https://nixiesearch.substack.com/p/benchmarking-api-latency-of-embedding)
- [fastembed-rs (Rust ONNX embeddings)](https://github.com/Anush008/fastembed-rs)

### Edit Formats
- [Aider: Edit Formats](https://aider.chat/docs/more/edit-formats.html)
- [Aider: Unified Diffs Make GPT-4 Turbo 3X Less Lazy](https://aider.chat/docs/unified-diffs.html)
- [Code Surgery: How AI Assistants Make Precise Edits (Hertwig, 2025)](https://fabianhertwig.com/blog/coding-assistants-file-edits/)
- [OpenAI: GPT-4.1 Prompt Cookbook](https://developers.openai.com/api/docs/guides/predicted-outputs/)
- [Anthropic: Structured Outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs)

### Chunking & Retrieval
- [Beyond Chunk-Then-Embed (2025)](https://arxiv.org/html/2602.16974)
- [Jina: Late Chunking (2024)](https://arxiv.org/abs/2409.04701)
- [Pinecone: Chunking Strategies](https://www.pinecone.io/learn/chunking-strategies/)
- [Cursor: Semantic Search](https://cursor.com/docs/context/semantic-search)
- [Redis: LLM Token Optimization (2026)](https://redis.io/blog/llm-token-optimization-speed-up-apps/)

### Vector Similarity in Rust
- [SimSIMD (SIMD-accelerated similarity)](https://github.com/ashvardanian/SimSIMD)
- [The State of SIMD in Rust (2025)](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d)
- [Sentence Transformers: Semantic Search](https://sbert.net/examples/applications/semantic-search/README.html)
