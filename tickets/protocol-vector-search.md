# Ticket: Protocol Vector Search

## Summary

Vectorize protocol markdown files at startup so user box text can be matched to the correct protocol via semantic search.

## Context

Protocols live in `config/archetype/` as markdown files (currently only `workforce`). When a user writes text in a board node (e.g., "Have a meeting about X"), the system needs to select the right protocol. Instead of LLM reasoning or keyword matching, embed the protocol descriptions and cosine-match against the user's query.

## Current State

- One protocol exists: `config/archetype/workforce/` (archetype.md, agent/, builder/)
- Protocol configs are loaded at compile time via `include_str!()` in `src/config/protocols.rs`
- Ollama is already wired into the LLM provider registry (`src/llm/`)

## Design

### Startup

1. Load each protocol's markdown description (e.g., `config/archetype/workforce/archetype.md`)
2. Embed each via Ollama (`nomic-embed-text` or `all-minilm`)
3. Store in memory: `Vec<(protocol_name, embedding_vector)>`

### Runtime

1. User writes text in a board node → `raw_text` captured by board serializer
2. Embed the user's text via Ollama
3. Cosine similarity against stored protocol vectors
4. Select highest-scoring protocol above a confidence threshold
5. Apply selected protocol to the node/step

### Architecture

```
config/archetype/workforce/archetype.md  →  embed()  →  stored in AppState
config/archetype/meeting/archetype.md    →  embed()  →  stored in AppState
                                                         ↑
user box text: "Have a meeting about X"  →  embed()  →  cosine search  →  "meeting"
```

### Notes

- Brute-force cosine search is fine — protocol count will stay small (<100)
- No vector database needed — in-memory `Vec` with linear scan
- Ollama embedding models are fast (~5ms per query) and free
- Embedding model: `nomic-embed-text` (768 dims) or `all-minilm` (384 dims)
- Consider fallback: if no protocol scores above threshold, ask the user or default to workforce

## Files

- `src/config/protocols.rs` — protocol loading (currently `include_str!`)
- `config/archetype/*/archetype.md` — protocol descriptions to embed
- `src/llm/` — Ollama provider (already exists)
- `src/server/state/` — AppState (store embeddings here)
- New: embedding service module for embed + search logic
