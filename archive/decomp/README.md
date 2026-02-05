# Decomposition Files

This directory contains detailed ticket breakdowns created by the Orchestrator.

## Structure

```
decomp/
├── M1/          ← Milestone 1: Foundation
│   ├── 1.1.md   ← Ticket 1.1: Project Scaffolding
│   ├── 1.2.md   ← Ticket 1.2: Core Type Definitions
│   └── ...
├── M2/          ← Milestone 2: LLM Layer
│   ├── 2.1.md
│   └── ...
└── ...
```

## Usage

**Orchestrator creates these:**
```
YOUR TASK: Milestone 1
PLEASE SEE: ORCHESTRATOR.md
```

**Workers consume these:**
```
YOUR TASK: Ticket 1.2
PLEASE SEE: WORKER.md, decomp/M1/1.2.md
```

## File Format

Each ticket file contains:
- Goal (what "done" looks like)
- Context (background info)
- Slices (step-by-step breakdown)
- Dependencies (what must exist first)
- Verification steps
