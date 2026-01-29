# Ticket 5.0: Planner Bot (Interactive PRD Creation)

> A specialized bot for `/plan` mode that helps users create PRDs through conversation.

## Goal

Implement an interactive planning assistant that guides users through creating Product Requirements Documents (PRDs) via a conversational interface. The Planner Bot operates in `/plan` mode and helps users scope projects, make technical decisions, and define milestones through a structured, phase-based conversation.

**Checkpoint**: Enter `/plan` mode, describe a project idea, have a multi-turn conversation with the Planner Bot across all phases (Discovery → Scoping → Technical → Milestones → Review), and produce a structured PRD document that can be saved to the database and exported to markdown format.

---

## Context

The Planner Bot is one of the specialized agents in the nexor system. While the Orchestrator (in `/main` mode) manages work execution, the Planner Bot focuses exclusively on helping users plan projects before execution begins.

### Key Differences from Orchestrator

| Aspect | Planner Bot | Orchestrator |
|--------|-------------|--------------|
| Mode | `/plan` | `/main` |
| Purpose | PRD creation | Work execution |
| Interaction | Multi-turn conversation | Task delegation |
| Output | PRD document | Completed tickets |
| Persona | Methodical, asks questions | Directive, assigns work |

### Planning Phases

The Planner Bot guides users through 5 phases:

1. **Discovery** - Understanding the problem and users
2. **Scoping** - Defining boundaries and scale
3. **Technical** - Making technology and architecture decisions
4. **Milestones** - Breaking project into deliverable phases
5. **Review** - Final review and approval

**Key files**:
- `src/types/prd.rs` - PRD document types (to be created)
- `src/agents/planner_bot.rs` - Planner Bot implementation (to be created)
- `src/agents/mod.rs` - Agent exports (extend)
- `src/db/prd.rs` - PRD persistence (to be created)
- `migrations/XXXXXX_add_prds.sql` - Database schema (to be created)

**Dependencies**:
- Requires M2 (LLM Layer) - LLMProvider for conversations
- Requires M3 (Agent Runtime) - Agent lifecycle management
- Requires M4 (Prompt Engineering) - Planner Bot persona templates
- Requires M1 (Foundation) - Database connection pool

**References**:
- See `decomp/M5/5.0.md` for full specification
- See `decomp/M5/README.md` for context on planning vs. execution
- See `PRD.md` "Decomposition Guide" for PRD structure examples

---

## Slices

### Slice 5.0.1: Define PRD Document Types

**Do this**:
- Create `src/types/prd.rs` with PRD document structure
- Define `PRDDocument` struct with all required fields (vision, milestones, technical decisions, etc.)
- Define `PRDStatus` enum (Draft, Review, Approved, Archived)
- Define `MilestoneSpec`, `TechnicalDecision`, `DataModelSketch` structs
- Implement `PRDDocument::new()` constructor
- Implement `is_complete()` validation method
- Implement `estimated_scale()` helper (Feature/Project/Epic based on milestone count)
- Add serde serialization for JSON persistence
- Export from `src/types/mod.rs`

**Create/modify**:
- `src/types/prd.rs` (create new file)
- `src/types/mod.rs` (add `pub mod prd;`)

**Verify**:
- [ ] `cargo check` passes
- [ ] Types serialize/deserialize to JSON correctly
- [ ] `is_complete()` returns false for empty PRD
- [ ] `is_complete()` returns true when vision and milestones exist
- [ ] `estimated_scale()` correctly categorizes by milestone count

---

### Slice 5.0.2: Create Planner Bot Persona

**Do this**:
- Create `src/agents/planner_bot.rs`
- Define `PlannerBot<P: LLMProvider>` struct
- Define `PlanningSession` to track conversation state
- Define `PlanningPhase` enum (Discovery, Scoping, Technical, Milestones, Review)
- Define `PlanningMessage` for conversation history
- Implement `PlannerBot::new()` constructor
- Implement `PlannerBot::start_session()` to initialize a new planning conversation
- Implement `system_prompt()` with phase-specific guidance
- Create persona that is methodical, asks clarifying questions, and pushes back on scope creep
- Export from `src/agents/mod.rs`

**Create/modify**:
- `src/agents/planner_bot.rs` (create new file)
- `src/agents/mod.rs` (add `pub mod planner_bot;`)

**Verify**:
- [ ] `cargo check` passes
- [ ] System prompt includes personality traits
- [ ] System prompt includes phase-specific guidance for all 5 phases
- [ ] Session initializes with Discovery phase
- [ ] Session tracks PRD state and current phase

---

### Slice 5.0.3: Implement Conversation Loop

**Do this**:
- Implement `PlannerBot::chat()` method for single-turn conversation
- Build conversation history into LLM request messages
- Add user message to session history before LLM call
- Add assistant response to session history after LLM call
- Implement `build_messages()` to convert session history to LLM format
- Implement `process_response()` to detect phase transitions
- Implement `extract_json_block()` to parse structured outputs
- Implement `apply_structured_update()` to update PRD from JSON blocks
- Define `PlannerBotError` error type with LlmError, ParseError, SessionError variants
- Support automatic phase transitions based on response content

**Create/modify**:
- `src/agents/planner_bot.rs` (extend)

**Verify**:
- [ ] `cargo check` passes
- [ ] Conversation history builds correctly with system + user + assistant messages
- [ ] Phase transitions detected from keywords ("moving to scoping", "let's discuss technical", etc.)
- [ ] JSON blocks extracted from ```json...``` code fences
- [ ] MilestoneSpec and TechnicalDecision updates applied to PRD
- [ ] Session updated_at timestamp updated on PRD changes

---

### Slice 5.0.4: PRD Finalization and Export

**Do this**:
- Implement `PlannerBot::finalize_prd()` to mark PRD as approved
- Validate PRD is complete before finalization (has vision + milestones)
- Return error if validation fails
- Update PRD status to Approved and timestamp
- Implement `PlannerBot::export_markdown()` to generate markdown output
- Include all PRD sections in markdown: title, status, vision, problem statement, success criteria, technical decisions table, milestones
- Format milestones with proper heading levels and dependency references
- Return completed PRD document on successful finalization

**Create/modify**:
- `src/agents/planner_bot.rs` (extend)

**Verify**:
- [ ] `cargo check` passes
- [ ] Finalization fails with error if PRD incomplete
- [ ] Finalization succeeds and sets status to Approved
- [ ] Markdown export produces valid, readable format
- [ ] Markdown includes all non-empty sections
- [ ] Technical decisions rendered as table
- [ ] Milestones show dependencies correctly

---

### Slice 5.0.5: Database Persistence

**Do this**:
- Create migration `migrations/014_add_prds.sql` for PRD tables
- Create `prds` table with all PRD fields (use JSON for arrays)
- Create `planning_sessions` table for resumable conversations
- Add indexes for status and session lookups
- Create `src/db/prd.rs` with database operations
- Implement `save_prd()` with INSERT ... ON CONFLICT UPDATE pattern
- Implement `load_prd()` to retrieve by ID
- Implement `list_prds_by_status()` to query by status
- Parse status strings back to PRDStatus enum
- Parse RFC3339 timestamps back to DateTime<Utc>
- Deserialize JSON arrays for success_criteria, technical_decisions, etc.
- Export from `src/db/mod.rs`

**Create/modify**:
- `migrations/014_add_prds.sql` (create new file)
- `src/db/prd.rs` (create new file)
- `src/db/mod.rs` (add `pub mod prd;`)

**Verify**:
- [ ] `cargo check` passes
- [ ] Migration runs successfully with `sqlx migrate run`
- [ ] PRDs can be saved to database
- [ ] PRDs can be loaded from database by ID
- [ ] PRD data round-trips correctly (save then load produces identical data)
- [ ] Status enum serializes as string and deserializes correctly
- [ ] Timestamps preserve timezone information
- [ ] JSON arrays deserialize with error handling (unwrap_or_default)

---

## Notes

### Persona Design

The Planner Bot has a distinct personality:
- **Methodical**: Moves through phases systematically
- **Inquisitive**: Asks clarifying questions rather than assume
- **Realistic**: Pushes back on unrealistic scope or timelines
- **Focused**: Keeps conversation on current phase, prevents jumping ahead
- **Concrete**: Prefers specific details over vague descriptions

### Conversation Flow

```
User: "I want to build a task management app"
        ↓
Planner (Discovery): "Great! Let me understand the problem first.
                      Who are the target users? Are they individuals
                      or teams? What pain point does this solve?"
        ↓
User provides details
        ↓
Planner: "I see. Let me summarize... [recap]. Does that capture it?
          Moving to scoping..."
        ↓
Planner (Scoping): "For v1, should we focus on personal task tracking
                    or include team collaboration features?"
        ↓
... continues through phases ...
```

### Integration with Orchestrator

Once approved, PRDs feed into the main execution flow:

1. User completes PRD in `/plan` mode
2. PRD saved to database with status=Approved
3. User switches to `/main` mode
4. Orchestrator can load PRD: "Execute the task management app PRD"
5. Orchestrator converts PRD milestones into tickets
6. Planner (5.1) decomposes tickets into slices
7. Normal work execution begins

### Technical Considerations

- **State Management**: Sessions must be resumable if user exits `/plan` mid-conversation
- **Validation**: Don't allow finalization of incomplete PRDs
- **Flexibility**: Some PRD fields are optional (data models, effort estimates)
- **JSON Storage**: Arrays stored as JSON TEXT in SQLite for simplicity
- **Phase Transitions**: Detected via keyword matching in responses (simple but effective)

---

## Completion Checklist

Before marking this ticket done:

- [ ] All 5 slices verified
- [ ] `cargo check` passes
- [ ] `cargo test` passes (if tests exist)
- [ ] PRD types serialize/deserialize correctly
- [ ] Conversation flow moves through all 5 phases
- [ ] Phase-specific system prompts are distinct
- [ ] PRD validation works (is_complete)
- [ ] PRD export produces valid markdown
- [ ] Database migration runs successfully
- [ ] PRDs persist and load correctly
- [ ] Status transitions work (Draft → Review → Approved)
- [ ] Code follows `CONVENTIONS.md`
- [ ] `PROGRESS.md` updated with ticket completion

---

## Future Enhancements (Out of Scope)

These are NOT required for M5.0 but could be added later:

- Template library for common project types (API, CLI, web app, etc.)
- PRD comparison/diff view for revisions
- Collaborative planning (multiple users)
- AI-suggested milestones based on project description
- PRD export to other formats (PDF, HTML)
- Integration with external planning tools (Jira, Linear, etc.)
