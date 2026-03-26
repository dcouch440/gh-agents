# System Node Agent Stress Test 2 — No Web Search

## Workflow: Micro SaaS Product Spec

7 nodes. Double fan-out, diamond merge, linear tail. Tests deep dependency chains, parallel coordination, and structured document synthesis.

```
[Product Idea] → [User Personas]     → [Feature Matrix]  → [Launch Checklist]
               → [Market Landscape]  ↗                    ↗
               → [Pricing Strategy]  → [Revenue Model]  ─┘
```

## Node text (paste each as a box on the canvas)

### Node 1: Product Idea
```
Invent a micro SaaS product for freelance developers. It should solve a real pain point. Write a one-page product brief: problem statement, proposed solution, target user, and a catchy product name.
```

### Node 2: User Personas (depends on Node 1)
```
Create 3 detailed user personas for this product. Each persona should have: name, role, daily workflow, biggest frustration, and how this product helps them. Write as structured profiles.
```

### Node 3: Market Landscape (depends on Node 1)
```
Analyze the competitive landscape for this product. Invent 4 plausible competitors with names, pricing, strengths, and weaknesses. Position our product against them. No web search needed — make it realistic but fictional.
```

### Node 4: Pricing Strategy (depends on Node 1)
```
Design a pricing strategy with 3 tiers (free, pro, team). For each tier: name, monthly price, included features, usage limits, and target persona. Include a brief rationale for the pricing structure.
```

### Node 5: Feature Matrix (depends on Node 2 AND Node 3)
```
Build a feature priority matrix combining persona needs and competitive gaps. Rank features as Must-Have, Should-Have, or Nice-to-Have. Output as a structured table with columns: Feature, Priority, Persona Served, Competitive Advantage.
```

### Node 6: Revenue Model (depends on Node 4)
```
Build a 12-month revenue projection spreadsheet (as a markdown table). Assume: Month 1 starts with 50 free users, 5% monthly conversion to pro, 10% monthly growth. Show columns: Month, Free Users, Pro Users, Team Users, MRR, Cumulative Revenue.
```

### Node 7: Launch Checklist (depends on Node 5 AND Node 6)
```
Create a detailed launch checklist combining the feature priorities and revenue targets. Group items into: Pre-Launch (weeks 1-2), Launch Week, Post-Launch (weeks 3-4). Each item should reference which feature or revenue milestone it supports.
```

## Topology

```
Node 1 ──→ Node 2 ──→ Node 5 ──→ Node 7
       ├──→ Node 3 ──↗
       └──→ Node 4 ──→ Node 6 ──↗
```

- Level 0: Node 1 (root)
- Level 1: Node 2, Node 3, Node 4 (triple fan-out, all parallel)
- Level 2: Node 5 (fan-in from 2+3), Node 6 (linear from 4) — parallel
- Level 3: Node 7 (fan-in from 5+6)

## What this tests beyond Test 1

| Challenge | How it's tested |
|-----------|-----------------|
| **Triple fan-out** | Node 1 feeds 3 parallel nodes (not just 2) |
| **Double diamond** | Two separate fan-in points (Node 5 and Node 7) |
| **Independent parallel chains** | Node 4→6 runs independently from Node 2+3→5 |
| **Level 2 parallelism** | Node 5 and Node 6 can run in parallel (no dependency) |
| **Deep chain** | Node 7 is 3 hops from root (1→2→5→7 or 1→4→6→7) |
| **Structured data** | Feature matrix (table), revenue model (spreadsheet), checklist (grouped) |
| **Cross-referencing** | Node 7 must reference BOTH features and revenue — different document types |
| **No web search** | Everything is generated — tests pure reasoning without external tools |
| **7 files total** | 3 parallel writes at Level 1, 2 parallel writes at Level 2 |
