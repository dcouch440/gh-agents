# System Node Agent Stress Test

## Workflow: Competitive Product Analysis

5 nodes. Fan-out, fan-in, diamond dependency. Tests parallel execution, file coordination, and multi-source merge.

```
[Company Brief] → [Web Research]    → [Comparison Matrix]  → [Executive Brief]
                → [Product Teardown] ↗
```

## Node text (paste each as a box on the canvas)

### Node 1: Company Brief
```
Write a brief company profile for Notion (the productivity app). Include: what they do, target market, pricing tiers, and key differentiators. Keep it to one page.
```

### Node 2: Web Research (depends on Node 1)
```
Research Notion's top 3 competitors. For each competitor, find: name, pricing, key features, and one weakness compared to Notion.
```

### Node 3: Product Teardown (depends on Node 1)
```
Analyze Notion's product from a UX perspective. Evaluate: onboarding flow, learning curve, mobile experience, and collaboration features. Rate each area 1-5.
```

### Node 4: Comparison Matrix (depends on Node 2 AND Node 3)
```
Build a structured comparison matrix combining the competitor research and the UX analysis into one table. Include Notion and all competitors with scores across features, pricing, and UX.
```

### Node 5: Executive Brief (depends on Node 4)
```
Write a 1-page executive brief with a recommendation: should a startup adopt Notion? Use the comparison matrix as evidence. Include pros, cons, and a final verdict.
```

## What this tests

| Challenge | How it's tested |
|-----------|-----------------|
| **Fan-out** | Node 1 feeds both Node 2 and Node 3 in parallel |
| **Fan-in** | Node 4 must read outputs from BOTH Node 2 and Node 3 |
| **File coordination** | Node 2 and Node 3 run in parallel, write different files |
| **Multi-hop context** | Node 5 depends on Node 4 which depends on 2+3 which depend on 1 |
| **Structured output** | Node 4 must produce a table/matrix, not just prose |
| **Synthesis** | Node 5 must synthesize structured data into a recommendation |
| **Web search** | Node 2 needs real competitor data (tests web_search tool) |

## Expected agent topology per node

- Node 1: 1 agent (writer)
- Node 2: 1 agent (researcher) — should use web search
- Node 3: 1 agent (analyst)
- Node 4: 1 agent (analyst) — reads 2 upstream files
- Node 5: 1 agent (writer) — reads matrix, produces recommendation

## What to verify in DISPATCH.json

- [ ] Each system node agent completes in 2 tool calls (run_command + complete_system)
- [ ] Descriptions flow correctly via `<previous_step>` blocks
- [ ] No node has more than 1 agent (simple tasks)
- [ ] expected_output tells agents to report filenames

## What to verify in RUN.json

- [ ] Node 2 and Node 3 run in parallel (check timestamps if available)
- [ ] Node 2 and Node 3 produce uniquely named files (no collision)
- [ ] Node 4 reads BOTH upstream files (check run_command for two cat calls)
- [ ] Node 4 produces a structured table (not just prose)
- [ ] Node 5 references the matrix in its recommendation
- [ ] All results show `"result: success"` (no false failures)
- [ ] Total files on disk: 5 (one per node)
