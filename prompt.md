# Research: Cosine Similarity Tool Selection for LLM Conversations

## Problem

An LLM agent has 13 tools available. Sending all 13 tool definitions on every turn wastes context window, increases latency, and confuses the model when most tools are irrelevant to the current message. We need a way to dynamically select a subset of tools per turn based on the user's message using cosine similarity between message embeddings and pre-computed tool description embeddings.

## What We Need To Know

### Part 1 — Landscape Research

Do a broad sweep of how people are solving dynamic tool selection today. This is one research effort, not separate tracks. Cover all of the following together:

- **Existing projects and papers**: Find open source implementations, frameworks, and academic work on dynamic tool selection, tool routing, and function calling optimization for LLM agents. Gorilla, Semantic Router, SERA, and anything else that's out there. What actually works in production vs what's theoretical?
- **Community discussion**: Search X (via Grok) for real conversations about tool selection — what are developers building, what problems are they hitting, what's the sentiment? Look for specific experience reports, not just announcements. Search terms: "tool selection LLM", "function calling optimization", "dynamic tool routing", "too many tools", "tool retrieval agent".
- **Embedding models for short functional text**: Which embedding models work best for tool descriptions specifically? Compare OpenAI text-embedding-3-small, bge-small-en-v1.5, all-MiniLM-L6-v2, Cohere embed-v3. We care about quality on short text (50-200 tokens), cost, and whether self-hosted is viable.
- **How to write tool descriptions that embed well**: Should we embed name only, description only, name+description, or name+description+parameter schema? Declarative vs action-oriented phrasing? What makes two similar tools distinguishable in embedding space?

### Part 2 — Architecture Recommendation

Based on the landscape findings, design the actual system for a 13-tool agent. Be specific and opinionated:

- **Selection strategy**: Threshold, top-k, or hybrid? What threshold value? What k? Should some tools be pinned (always included)? What's the fallback when nothing matches?
- **Multi-turn handling**: How does tool selection work across a conversation? Does the previous turn's tool usage influence the next selection? How do you handle "use that tool again" type messages?
- **Failure modes**: What specifically breaks — hallucinated tool calls, missed relevant tools, behavior differences at different tool counts (3 vs 8 vs 13)? How do you detect and recover from bad selections?
- **Implementation sketch**: Pseudocode for the full flow — startup precomputation, per-turn selection, pinned tools, fallback. What libraries, what data structures, what's the latency budget?
- **Evaluation**: How do you know if it's working? What metrics, what test set, how do you catch regressions?

## Deliverable Format

Write the report with full detail — specific numbers, specific links, specific code. Do not summarize findings into bullet points. If you found a paper, explain what it says. If you found a repo, explain how it works. If someone on X described a problem, quote them. Preserve the evidence, don't compress it.

The recommendation section should be a concrete architecture document someone could implement from, not a menu of options.
