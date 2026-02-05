# Milestone 4: Prompt Engineering & Agent Intelligence

> Build robust, tested prompts that drive reliable agent behavior.

## Goal

Create a comprehensive prompt system where agents consistently produce structured output, think step-by-step, and recover from confusion. This milestone treats prompts as engineering artifacts - versioned, tested, and debuggable.

**Checkpoint**: Run a decomposition prompt against a test ticket and verify:
1. Output matches the defined JSON schema
2. Reasoning is visible and follows the thinking pattern
3. Recovery prompt successfully reformats malformed output
4. Context stays within token budget

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 4.1 | Prompt Architecture Design | 4 | None (can start immediately) |
| 4.2 | Orchestrator Thinking Patterns | 5 | 4.1 |
| 4.3 | Worker Thinking Patterns | 5 | 4.1 |
| 4.4 | Utility Thinking Patterns | 4 | 4.1 |
| 4.5 | Structured Output Design | 5 | 4.1 |
| 4.6 | Few-Shot Examples Library | 5 | 4.2, 4.3, 4.4 |
| 4.7 | Prompt Testing Framework | 6 | 4.5, M2 (LLM Layer) |
| 4.8 | Context Management Strategy | 5 | 4.1 |
| 4.9 | Self-Correction & Recovery Prompts | 5 | 4.5 |
| 4.10 | Tool Definition & Selection | 6 | 4.1 |
| 4.11 | Context Window Validation | 5 | M2 (LLM Layer) |

**Total**: 11 tickets, 55 slices

---

## Dependency Graph

```
                    ┌─────┐
                    │ 4.1 │  Prompt Architecture Design
                    └──┬──┘
         ┌────────────┼────────────┬───────────┬───────────┐
         ▼            ▼            ▼           ▼           ▼
      ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐
      │ 4.2 │     │ 4.3 │     │ 4.4 │     │ 4.5 │     │ 4.8 │
      └──┬──┘     └──┬──┘     └──┬──┘     └──┬──┘     └─────┘
         │           │           │           │
         └─────┬─────┴───────────┘           │
               ▼                             │
            ┌─────┐                          │
            │ 4.6 │                          │
            └─────┘                          │
                              ┌──────────────┴──────────────┐
                              ▼                             ▼
                           ┌─────┐                       ┌─────┐
                           │ 4.9 │                       │ 4.7 │
                           └─────┘                       └─────┘
                                                            │
                                                      (needs M2)

      ┌─────┐
      │ 4.10│  (depends on 4.1)
      └─────┘

      ┌─────┐
      │ 4.11│  (depends on M2 only)
      └─────┘
```

---

## Parallelization

**Can run in parallel (after 4.1 complete)**:
- 4.2, 4.3, 4.4 (thinking patterns for each tier)
- 4.5 (structured output)
- 4.8 (context management)
- 4.10 (tool definitions)

**Can run in parallel (once M2 complete)**:
- 4.7 and 4.11 only need M2, not other M4 tickets

**Must be sequential**:
- 4.1 must complete first (foundation for all others)
- 4.6 needs 4.2, 4.3, 4.4 complete (examples based on thinking patterns)
- 4.9 needs 4.5 complete (recovery needs output schemas)
- 4.7 needs 4.5 and M2 (testing framework needs schemas and LLM)

**Recommended execution order**:
1. Start with 4.1 (Prompt Architecture Design)
2. After 4.1: Start 4.2, 4.3, 4.4, 4.5, 4.8, 4.10 in parallel
3. After 4.2, 4.3, 4.4: Start 4.6 (Few-Shot Examples)
4. After 4.5: Start 4.9 (Self-Correction)
5. After 4.5 + M2 complete: Start 4.7 (Prompt Testing Framework)
6. After 4.8 + M2 complete: Start 4.11 (Context Window Validation)

---

## File Structure

All prompt engineering code will live in a new `src/prompts/` directory:

```
src/prompts/
├── mod.rs                   ← Module exports
├── builder.rs               ← 4.1: PromptBuilder
├── version.rs               ← 4.1: Prompt versioning
├── templates/
│   ├── mod.rs
│   ├── orchestrator.rs      ← 4.2: Orchestrator prompts
│   ├── worker.rs            ← 4.3: Worker prompts
│   └── utility.rs           ← 4.4: Utility prompts
├── schemas/
│   ├── mod.rs
│   ├── decomposition.rs     ← 4.5: Decomposition output
│   ├── task_result.rs       ← 4.5: Task result output
│   ├── review.rs            ← 4.5: Review output
│   └── error.rs             ← 4.5: Error output
├── examples/
│   ├── mod.rs
│   ├── decomposition.rs     ← 4.6: Decomposition examples
│   ├── implementation.rs    ← 4.6: Implementation examples
│   ├── review.rs            ← 4.6: Review examples
│   └── selector.rs          ← 4.6: Example selection logic
├── context/
│   ├── mod.rs
│   ├── manager.rs           ← 4.8: Context budget/selection
│   ├── summarizer.rs        ← 4.8: Large file summarization
│   └── validator.rs         ← 4.11: Token counting/limits
├── recovery/
│   ├── mod.rs
│   └── prompts.rs           ← 4.9: Recovery prompt templates
└── tools/
    ├── mod.rs
    └── definitions.rs       ← 4.10: Tool schemas

tests/
└── prompts/
    ├── mod.rs
    ├── harness.rs           ← 4.7: Test harness
    ├── assertions.rs        ← 4.7: Assertion library
    └── fixtures/            ← 4.7: Test fixtures
        ├── tickets/
        └── expected_outputs/
```

---

## Agent Tier Assignments

| Ticket | Recommended Tier | Rationale |
|--------|------------------|-----------|
| 4.1 | Orchestrator | Architectural design work |
| 4.2 | Orchestrator | Designing own thinking patterns |
| 4.3 | Worker | Can implement from 4.2 as template |
| 4.4 | Worker | Can implement from 4.2 as template |
| 4.5 | Worker | Schema design, straightforward |
| 4.6 | Orchestrator | Requires judgment for good examples |
| 4.7 | Worker | Framework implementation |
| 4.8 | Worker | Implementation from design |
| 4.9 | Worker | Implementation from schemas |
| 4.10 | Worker | Tool definition implementation |
| 4.11 | Worker | Token counting implementation |

---

## Notes

### Why a dedicated milestone?

Prompts ARE the behavior. A poorly-crafted prompt means an unreliable agent. This milestone is about engineering the thought process, not just writing text.

### Key principles from ROADMAP.md

1. **Show, don't tell** - Examples beat instructions
2. **Make thinking visible** - Explain reasoning before conclusions
3. **Fail loudly, recover gracefully** - Say "I'm confused" not silently guess
4. **Structured output is non-negotiable** - JSON for machines, natural language for humans
5. **Context is precious** - Every token should earn its place
6. **Test prompts like code** - Version, test, and diff them

### External dependencies

- **M2 (LLM Layer)** needed for 4.7 and 4.11 to actually call the LLM
- Most M4 tickets can start without M2 (design and implementation work)
- Integration testing of prompts requires M2 complete

### Validation approach

Unlike code that compiles, prompts need LLM calls to validate. Plan for:
- Mocked LLM responses for unit tests (fast, deterministic)
- Real LLM calls for integration tests (slow, verify actual behavior)
- Regression suite to detect prompt changes that affect output

---

## Extensions Required for Plan Mode (M5.0)

Plan Mode (ticket 5.0) requires additional prompts and schemas. These should be added as extensions to the existing tickets:

### Ticket 4.2 Extensions (Orchestrator Thinking Patterns)

Add new thinking patterns for Plan Mode:

| Pattern | Purpose |
|---------|---------|
| **Request Classification** | Determine scale: Quick, Task, Feature, Project, Epic |
| **PRD Generation** | Generate product requirements for Project/Epic scale |
| **Epic Decomposition** | Break epic into milestones with dependencies |

**New file**: `src/prompts/templates/planning.rs`

### Ticket 4.5 Extensions (Structured Output Design)

Add new output schemas:

```rust
// Classification output
struct ClassificationOutput {
    scale: String,           // "quick" | "task" | "feature" | "project" | "epic"
    confidence: f32,         // 0.0-1.0
    reasoning: String,
    suggested_title: String,
    indicators: Vec<String>,
}

// PRD output
struct PrdOutput {
    title: String,
    vision: String,
    milestones: Vec<MilestoneOutput>,
    technical_decisions: Vec<String>,
    data_models: Vec<String>,
}

struct MilestoneOutput {
    number: u32,
    title: String,
    goal: String,
    checkpoint: String,
    estimated_tickets: u32,
    dependencies: Vec<u32>,
}
```

**New file**: `src/prompts/schemas/planning.rs`

### Ticket 4.6 Extensions (Few-Shot Examples)

Add examples for Plan Mode:

| Example Type | Count | Purpose |
|--------------|-------|---------|
| Classification | 5 | One per scale level |
| Mini-PRD | 2-3 | Project-level examples |
| Full PRD | 2-3 | Epic-level examples |

**New file**: `src/prompts/examples/planning.rs`

### Updated File Structure

```
src/prompts/
├── templates/
│   ├── orchestrator.rs
│   ├── worker.rs
│   ├── utility.rs
│   └── planning.rs        ← NEW: Plan Mode prompts
├── schemas/
│   ├── decomposition.rs
│   ├── task_result.rs
│   ├── review.rs
│   ├── error.rs
│   └── planning.rs        ← NEW: Classification, PRD schemas
└── examples/
    ├── decomposition.rs
    ├── implementation.rs
    ├── review.rs
    ├── selector.rs
    └── planning.rs        ← NEW: Plan Mode examples
```

### Implementation Note

These extensions can be implemented as part of the existing tickets (4.2, 4.5, 4.6) or as a separate "4.12: Plan Mode Prompts" ticket. The latter is cleaner but adds another ticket to track.
