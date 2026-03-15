# Test: Multi-Agent Parallel Nodes with Fan-In

Draw three boxes. Arrow from Node 1 → Node 3, arrow from Node 2 → Node 3.

## Node 1
```
Research Tesla's latest quarterly earnings and financial performance
```

## Node 2
```
Research Rivian's latest quarterly earnings and financial performance
```

## Node 3 (receives from both)
```
Compare the two companies side by side and produce an investment analysis report with a recommendation
```

## What to verify

- Node 1 & 2: multi-agent teams (researcher + analyst or similar) with dependencies
- Intra-node agents pass data through the store (not response text)
- Node 3: fan-in consumer reads both upstream stores
- All configs use dual expected_output: "Store: [artifact]. Response: [summary]."
- No runtime block names referenced in any agent config
- Proportional prompts — complex tasks get 120-250 tokens, not more
- Store chain: intra-node (agent→agent) AND cross-node (node→node) all via files

---

# Test: Run-Scoped Artifacts (Re-execution Isolation)

Draw two boxes. Arrow from Node 1 → Node 2.

## Node 1
```
Scrape the top 5 stories from Hacker News right now and save a summary of each
```

## Node 2 (receives from Node 1)
```
Take the HN stories and rank them by potential business impact, produce a brief investment memo
```

## Test procedure

1. Submit the board — let designer phase complete
2. **Run 1** — execute the workflow. Node 1 produces store artifacts, Node 2 reads them.
3. **Run 2** — execute the workflow again WITHOUT resubmitting the board.
4. Check Node 2's `<upstream_artifacts>` block in Run 2 — it should ONLY contain files from Run 2, not Run 1's stale artifacts.

## What to verify

- Run 2's root node (Node 1) has NO `<upstream_artifacts>` section (no upstream + no stale files)
- Run 2's Node 2 sees ONLY files written by Node 1 during Run 2
- Design configs (`.system/design/`) persist across runs (workflow_run_id = NULL)
- Runtime artifacts have workflow_run_id set to the current run's ID
- No duplicate or stale file paths from Run 1 appear in Run 2's manifests
