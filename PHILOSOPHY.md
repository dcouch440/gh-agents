# Philosophy: Building with AI Agents

> How we build nexor, and how nexor will build software.

---

## The Meta Loop

We're building an AI agent orchestration system **using** AI agent orchestration.

```
How we build nexor                 What nexor will automate
──────────────────                 ────────────────────────
Human writes ROADMAP.md      →     User provides ticket/issue
AI orchestrator expands      →     Orchestrator agent decomposes
  into decomp/ files               into slices (stored in SQLite)
AI worker implements         →     Worker agents implement
PROGRESS.md (manual)         →     SQLite state (automatic)
```

**The flow today:**
1. Human writes high-level specs in `ROADMAP.md` (tickets + brief slices)
2. Orchestrator (AI) expands into detailed `decomp/` files with code examples
3. Worker (AI) implements slice by slice

The system we use to build is the system we're building. Every pain point we feel, every friction we smooth—that's product insight.

---

## Core Principles

### 1. Decomposition is Everything

Complex work becomes simple through decomposition:

```
Epic (months)
  → Milestones (weeks)
    → Tickets (days)
      → Slices (hours)
```

Each level answers a different question:
- **Epic**: What are we building and why?
- **Milestone**: What's the next usable checkpoint?
- **Ticket**: What's one complete feature?
- **Slice**: What's the smallest deployable unit?

**The magic is in the slices.** A slice must:
- Work independently
- Be verifiable
- Touch all necessary layers (vertical, not horizontal)
- Take hours, not days

### 2. Separation of Planning and Doing

Two distinct modes of work:

| Mode | Question | Output |
|------|----------|--------|
| **Planning** | "What should we build?" | Decomp files, specs, tickets |
| **Doing** | "How do I build this?" | Code, tests, commits |

Mixing these creates confusion. An agent in "doing" mode shouldn't be deciding what to do—that's already decided. An agent in "planning" mode shouldn't be writing code—they should be writing specs.

**Orchestrator = Planning. Worker = Doing.**

### 3. Documentation as Interface

AI agents don't have meetings. They don't overhear conversations. They only know what's written down.

```
Bad:  "You know how we discussed the auth approach..."
Good: "See decomp/M1/1.3.md for auth implementation spec"
```

Documentation isn't overhead—it's the interface between agents. Every decision, every context, every requirement must be written or it doesn't exist.

### 4. Explicit Over Implicit

Humans infer. AI agents follow instructions.

```
Bad:  "Make the config system robust"
Good: "Load ~/.config/nexor/config.toml, fall back to defaults if missing,
       validate required fields, return typed Config struct"
```

The decomp files must be explicit enough that a worker agent can execute without guessing. If they need to guess, the orchestrator didn't do their job.

### 5. Verify at Every Step

Trust nothing. Verify everything.

```
Slice 1.2.1: Create task types
  - Do this: Create src/types/task.rs with TaskStatus, Priority, Task
  - Verify: cargo check passes, can instantiate each type

Slice 1.2.2: Create agent types
  - Do this: Create src/types/agent.rs with Agent, AgentStatus
  - Verify: cargo check passes, can instantiate each type
```

Each slice has verification steps. Workers don't move forward until verification passes. This prevents error accumulation.

### 6. Progress is Visible

Everyone (human and AI) should be able to see:
- What's done
- What's in progress
- What's blocked
- What's next

`PROGRESS.md` is the single source of truth for status. Update it religiously.

---

## The Two Roles

### Orchestrator: The Architect

**Mindset**: "How do I break this down so a focused implementer can succeed?"

**Responsibilities**:
- Read high-level specs (ROADMAP.md)
- Decompose into detailed tickets
- Create clear, explicit decomp files
- Identify dependencies between tickets
- Make architectural decisions

**Does NOT**:
- Write implementation code
- Make decisions that should be in the spec
- Leave ambiguity for workers to resolve

### Worker: The Implementer

**Mindset**: "How do I execute this spec precisely and verify it works?"

**Responsibilities**:
- Read decomp files carefully
- Implement slice by slice
- Verify each slice before proceeding
- Update progress tracking
- Note blockers or issues

**Does NOT**:
- Decide what to build (that's in the decomp)
- Refactor unrelated code
- Add features not in the spec
- Skip verification steps

---

## Why This Works

### For AI Agents

1. **Bounded context**: Each agent works with a focused set of documents
2. **Clear instructions**: Decomp files are explicit specifications
3. **No ambiguity**: Decisions are made before implementation begins
4. **Verifiable progress**: Each slice has pass/fail criteria

### For Humans

1. **Visibility**: See exactly what agents are doing via PROGRESS.md
2. **Control**: Approve plans before implementation starts
3. **Quality**: Verification steps catch issues early
4. **Handoff**: Any agent can pick up any ticket—context is written

### For the Project

1. **Parallelization**: Independent tickets can be worked simultaneously
2. **Incremental progress**: Each slice is a working increment
3. **Reduced risk**: Small slices mean small failures
4. **Documentation**: The decomp files become project documentation

---

## The Feedback Loop

```
┌────────────────────────────────────────────────────────────┐
│                                                            │
│  1. Human writes vision (PRD, ROADMAP)                     │
│           ↓                                                │
│  2. Orchestrator decomposes into tickets                   │
│           ↓                                                │
│  3. Worker implements tickets                              │
│           ↓                                                │
│  4. Issues discovered → feed back to steps 1-2             │
│           ↓                                                │
│  5. Working software                                       │
│           ↓                                                │
│  6. Learn what worked → improve the process                │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

Every cycle teaches us:
- Where decomposition was too vague
- Where verification was insufficient
- Where dependencies were missed
- Where the process created friction

We fix the process, not just the code.

---

## Building nexor This Way

We're not just building software. We're building **the system that builds software**.

Every friction point we encounter is a feature opportunity:
- "I wish the agent knew X" → Add it to the context system
- "The decomp was unclear" → Improve orchestrator prompts
- "Progress tracking is tedious" → Automate it in the TUI
- "Verification is manual" → Build it into the execution layer

By building nexor with this manual process, we deeply understand what needs to be automated.

**The goal**: Eventually, the software we're building will do what we're doing now—automatically.

---

## Summary

| Principle | Why It Matters |
|-----------|----------------|
| Decomposition | Complex → Simple |
| Planning vs Doing | Clear responsibilities |
| Documentation as Interface | Agents only know what's written |
| Explicit over Implicit | No guessing allowed |
| Verify Every Step | Prevent error accumulation |
| Visible Progress | Everyone knows the state |

This isn't just a methodology. It's how intelligent systems collaborate on complex work.

We're proving it works by using it to build itself.
