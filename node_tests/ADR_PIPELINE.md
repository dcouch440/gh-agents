# System Node Agent Stress Test 3 — Code Review Pipeline

## Workflow: Architecture Decision Record

8 nodes. Fan-out into specialist reviewers, fan-in to conflict resolver, linear tail to final document. Tests multi-agent fan-in with disagreement handling and structured cross-referencing. No web search.

```
[RFC Draft] → [API Reviewer]        → [Conflict Resolver] → [ADR Document]
            → [Performance Reviewer] ↗                      ↗
            → [Security Reviewer]    ↗
            → [DX Reviewer]         → [Migration Plan]    ─┘
```

## Node text (paste each as a box on the canvas)

### Node 1: RFC Draft
```
Write a technical RFC proposing that our backend switches from REST to GraphQL for all client-facing APIs. Include: motivation (3 pain points with current REST API), proposed approach, API schema sketch for a "projects" resource with nested "tasks", and a list of open questions. Keep it to 2 pages. This is a fictional codebase — invent realistic details.
```

### Node 2: API Reviewer (depends on Node 1)
```
Review this RFC from an API design perspective. Evaluate: schema complexity, query depth limits, pagination strategy, error handling conventions, and backward compatibility with existing REST clients. For each area give a verdict (approve / concern / block) with a one-sentence rationale. End with an overall recommendation.
```

### Node 3: Performance Reviewer (depends on Node 1)
```
Review this RFC from a performance perspective. Analyze: N+1 query risk with nested resolvers, response payload sizes vs REST equivalents, caching strategy (CDN, persisted queries, dataloader), and expected latency impact. Quantify where possible with estimates. Give a verdict per area and an overall recommendation.
```

### Node 4: Security Reviewer (depends on Node 1)
```
Review this RFC from a security perspective. Evaluate: query complexity attacks (depth/breadth limiting), authorization model (field-level permissions vs resolver guards), introspection exposure in production, rate limiting strategy, and injection risks in custom scalars. Give a verdict per area and an overall recommendation.
```

### Node 5: DX Reviewer (depends on Node 1)
```
Review this RFC from a developer experience perspective. Evaluate: client code generation workflow, type safety improvements over REST, documentation tooling (playground, schema docs), testing strategy (mocking, integration tests), and migration burden for frontend teams. Give a verdict per area and an overall recommendation.
```

### Node 6: Conflict Resolver (depends on Node 2, Node 3, AND Node 4)
```
You have received reviews from API, Performance, and Security reviewers. Synthesize their feedback into a single conflict resolution document. For each area where reviewers disagree, state the conflict and propose a resolution. Where they agree, summarize the consensus. Produce a table: Area, API Verdict, Perf Verdict, Security Verdict, Resolution, Action Item.
```

### Node 7: Migration Plan (depends on Node 5)
```
Based on the DX review, create a phased migration plan from REST to GraphQL. Phase 1: dual-running period (4 weeks). Phase 2: new endpoints GraphQL-only (4 weeks). Phase 3: REST deprecation (8 weeks). For each phase list: deliverables, team responsibilities, rollback triggers, and success metrics. Output as a structured timeline.
```

### Node 8: ADR Document (depends on Node 6 AND Node 7)
```
Synthesize everything into a formal Architecture Decision Record. Use the standard ADR format: Title, Status (proposed), Context, Decision, Consequences (positive and negative), Compliance (how we'll enforce it), and Migration Reference (link the migration plan phases to specific consequences). Cross-reference the conflict resolution table and migration timeline by name.
```

## Topology

```
Node 1 ──→ Node 2 ──→ Node 6 ──→ Node 8
       ├──→ Node 3 ──↗
       ├──→ Node 4 ──↗
       └──→ Node 5 ──→ Node 7 ──↗
```

- Level 0: Node 1 (root)
- Level 1: Node 2, Node 3, Node 4, Node 5 (quad fan-out, all parallel)
- Level 2: Node 6 (triple fan-in from 2+3+4), Node 7 (linear from 5) — parallel
- Level 3: Node 8 (fan-in from 6+7)

## What this tests beyond Tests 1 and 2

| Challenge | How it's tested |
|-----------|-----------------|
| **Quad fan-out** | Node 1 feeds 4 parallel reviewers (most so far) |
| **Triple fan-in** | Node 6 merges 3 independent reviews (not just 2) |
| **Disagreement synthesis** | Node 6 must detect and resolve conflicting reviewer verdicts |
| **Asymmetric merge** | Node 8 merges outputs from different structures (table vs timeline) |
| **Role-based specialization** | 4 agents with distinct review lenses on the same input |
| **Cross-referencing by name** | Node 8 must reference specific items from Node 6's table and Node 7's phases |
| **Deep structured output** | ADR format, conflict table, phased timeline — all different document types |
| **Independent parallel chains** | Nodes 2+3+4→6 runs independently from Node 5→7 |
| **No web search** | Pure reasoning — fictional codebase, invented technical details |
| **8 agents total** | 4 parallel at Level 1, 2 parallel at Level 2 |
