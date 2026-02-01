# CLI Coverage Guide

## Running the Report

```bash
cd cli
npx vitest run --coverage
```

## What to Improve

Prioritize files by looking at the output table:

- **Low `% Lines` or `% Stmts`** — most impact
- **`Uncovered Line #s`** — tells you exactly which source lines need tests
- **Low `% Branch`** — missing `if/else`, error handling, or edge-case paths
