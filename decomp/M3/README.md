# Milestone 3: Agent Runtime

> Agents can be spawned, receive tasks, execute them, and report back.

## Goal

Establish the core agent infrastructure for nexor. After this milestone:
- Agents can be created with specific tiers (Orchestrator, Worker, Utility)
- Agent pools manage concurrent agents with configurable limits
- Agents communicate via typed message channels
- Agents have configurable personas that shape their behavior
- Agents can execute tasks by calling the LLM layer
- Failed tasks escalate up the tier hierarchy
- Inter-agent communication follows a well-defined protocol

**Checkpoint**: Can spawn a worker agent, give it a task, see it "work" (call LLM), report completion.

---

## Tickets

| Ticket | Title | Slices | Dependencies | Est. Complexity |
|--------|-------|--------|--------------|-----------------|
| 3.1 | Agent Struct & Lifecycle | 3 | M1 (types), M2 (LLM) | Medium |
| 3.2 | Agent Pool Manager | 4 | 3.1 | Medium |
| 3.3 | Message Passing | 4 | 3.1 | Medium |
| 3.4 | Persona System | 3 | M1 (config) | Low |
| 3.5 | Task Execution Loop | 4 | 3.1, 3.2, 3.3, 3.4 | High |
| 3.6 | Escalation Flow | 3 | 3.5 | Medium |
| 3.7 | Inter-Agent Protocol | 5 | 3.3 | Medium |

**Total Slices**: 26

---

## Dependency Graph

```
[M1 Foundation] ──────────────────────────────────────┐
      │                                               │
      │                                               │
[M2 LLM Layer] ──────┐                                │
      │              │                                │
      ▼              ▼                                ▼
   [3.1 Agent Struct & Lifecycle] ◄───────── [3.4 Persona System]
      │              │                                │
      │              │                                │
      ├──────────────┼────────────────────────────────┤
      │              │                                │
      ▼              ▼                                │
[3.2 Agent Pool] [3.3 Message Passing]                │
      │              │         │                      │
      │              │         │                      │
      │              │         ▼                      │
      │              │   [3.7 Inter-Agent Protocol]   │
      │              │                                │
      └──────────────┼────────────────────────────────┘
                     │
                     ▼
            [3.5 Task Execution Loop]
                     │
                     ▼
            [3.6 Escalation Flow]
```

**Simplified view:**

```
M1, M2 ──► 3.1 ──┬──► 3.2 ──────┐
                 │              │
                 ├──► 3.3 ──────┼──► 3.5 ──► 3.6
                 │    │         │
                 │    └──► 3.7  │
                 │              │
M1 config ──► 3.4 ──────────────┘
```

---

## Parallelization

**Can run in parallel:**
- 3.1 must complete first (core agent struct)
- Then: 3.2, 3.3, and 3.4 can run simultaneously
- 3.7 can start once 3.3 is done
- 3.5 must wait for 3.1, 3.2, 3.3, and 3.4
- 3.6 must wait for 3.5

**Optimal execution order:**
1. Start with 3.1 (agent struct)
2. After 3.1: Start 3.2, 3.3, and 3.4 in parallel
3. After 3.3: Start 3.7
4. After 3.2, 3.3, 3.4 complete: Start 3.5
5. After 3.5: Start 3.6

**Agent tier recommendations:**
| Ticket | Recommended Tier | Reason |
|--------|------------------|--------|
| 3.1 | Worker | Core struct, state machine logic |
| 3.2 | Worker | Pool management, concurrency |
| 3.3 | Worker | Channel setup, async patterns |
| 3.4 | Utility | Config loading, string templates |
| 3.5 | Worker | Main loop, LLM integration |
| 3.6 | Worker | Error handling, routing logic |
| 3.7 | Worker | Protocol design, validation |

---

## File Changes Summary

### New Files Created

```
nexor/
├── src/
│   └── agents/
│       ├── mod.rs                      ← 3.1.1 (update exports)
│       ├── agent.rs                    ← 3.1.1, 3.1.2, 3.1.3
│       ├── pool.rs                     ← 3.2.1, 3.2.2, 3.2.3, 3.2.4
│       ├── channels.rs                 ← 3.3.1, 3.3.2, 3.3.3, 3.3.4
│       ├── persona.rs                  ← 3.4.1, 3.4.2, 3.4.3
│       ├── executor.rs                 ← 3.5.1, 3.5.2, 3.5.3, 3.5.4
│       ├── escalation.rs               ← 3.6.1, 3.6.2, 3.6.3
│       └── protocol.rs                 ← 3.7.1, 3.7.2, 3.7.3, 3.7.4, 3.7.5
└── config/
    └── personas.toml                   ← 3.4.1 (embedded default personas)
```

### Modified Files

```
src/agents/mod.rs                       ← Add module exports
src/types/agent.rs                      ← May extend with runtime state
```

---

## Key Data Structures

From PRD.md and ROADMAP.md, these are the core types for this milestone:

```rust
// Agent with runtime state
struct Agent {
    id: AgentId,
    tier: AgentTier,
    persona: AgentPersona,
    model_config: ModelConfig,
    current_task: Option<Uuid>,
    status: AgentStatus,
    command_rx: mpsc::Receiver<AgentCommand>,
    response_tx: mpsc::Sender<AgentResponse>,
}

// Channel types
enum AgentCommand {
    AssignTask(TaskAssignment),
    RequestContext(ContextRequest),
    Shutdown,
}

enum AgentResponse {
    TaskResult(TaskResult),
    ContextResponse(ContextResponse),
    ProgressUpdate(ProgressUpdate),
    Error(AgentError),
}

// Protocol messages (from ROADMAP 3.7)
struct TaskAssignment {
    task_id: Uuid,
    title: String,
    description: String,
    context: TaskContext,
    constraints: TaskConstraints,
    timeout: Duration,
}
```

---

## Verification Checklist

After all tickets complete, verify:

- [ ] `cargo check` passes with no errors
- [ ] `cargo test` passes for all agent modules
- [ ] Can create an Agent with `Agent::new(tier, persona, config)`
- [ ] Agent state transitions work: Idle → Working → Idle
- [ ] `Agent::shutdown()` releases resources cleanly
- [ ] AgentPool respects max limits per tier
- [ ] `pool.spawn_agent(tier)` creates and tracks agents
- [ ] `pool.get_available_agent(tier)` returns idle agent
- [ ] Agents receive commands via mpsc channel
- [ ] Agents send responses back to dispatcher
- [ ] Default personas load from embedded config
- [ ] Project personas override defaults
- [ ] System prompts built correctly from persona + task
- [ ] Agent run loop processes tasks
- [ ] LLM is called during task execution
- [ ] Progress updates emitted to feed
- [ ] Task completion/failure handled correctly
- [ ] Failed tasks escalate up tiers
- [ ] "Needs human" terminal state works
- [ ] Protocol messages serialize/deserialize
- [ ] Invalid messages rejected with clear errors

---

## Notes

- This milestone builds the "brains" of the system - agents are the workers
- Heavily depends on M1 types and M2 LLM layer being complete
- The channel architecture is critical for concurrency - get it right
- Personas are more than cosmetic - they shape agent behavior
- The escalation flow is a safety mechanism - test it well
- Protocol messages are the contract between agents - validate strictly

---

## External Dependencies

This milestone requires:
- **M1: Foundation** - Core types (`AgentTier`, `AgentStatus`, `Task`, etc.)
- **M2: LLM Layer** - `LLMProvider` trait and Anthropic client for task execution

---

## Next Milestone

After M3, proceed to:
- **M5: Orchestration Core** - Uses agents for task decomposition and routing
- **M4: Prompt Engineering** - Can be done in parallel (prompt design doesn't need working agents)
