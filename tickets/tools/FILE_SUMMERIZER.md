       Read ALL of the following files and produce a detailed summary of each one, noting what was planned, what sections
       exist, and any status markers. I need to understand what's done vs what's remaining.

       Files to read:
       1. /Users/davidcouch/Dev/gh-agents/tickets/new-node-system/FULL_IMPLEMENTATION_PLAN.md
       2. /Users/davidcouch/Dev/gh-agents/tickets/new-node-system/BOCA_INTEGRATION_PLAN.md
       3. /Users/davidcouch/Dev/gh-agents/tickets/new-node-system/GENERALIZED_AGENT_DESIGNER_TICKET.md
       4. /Users/davidcouch/Dev/gh-agents/tickets/done/AGENT_DESIGNER_TICKET.md
       5. /Users/davidcouch/Dev/gh-agents/tickets/done/DYNAMIC_TASK_NODE_DESIGN.md
       6. /Users/davidcouch/Dev/gh-agents/tickets/done/EMBEDDED_USER_CONTEXT_TICKET.md
       7. /Users/davidcouch/Dev/gh-agents/tickets/done/NODE_ASSISTANT_PROMPTS.md

       For each file, provide:
       - File name
       - High-level purpose
       - Key sections/phases
       - Any completion status markers (checkboxes, "done", "completed", etc.)
       - What backend work it describes
       - What frontend work it describes
       - Whether it seems done, partially done, or not started

       1. /Users/davidcouch/Dev/gh-agents/tickets/new-node-system/FULL_IMPLEMENTATION_PLAN.md

       High-level purpose: The master implementation plan for the entire dynamic node system. This is the overarching
       roadmap that references the other design documents and organizes work into 11 phases.

       Key sections/phases:

       - Phase 1: Generalize the Archetype System -- Turn the documenter-specific step chat into a general archetype
       system. Includes base + archetype prompt system (1a), generalizing build_step_system_prompt() (1b), archetype
       catalog + step config API (1c), set_node_archetype tool (1d), verifying documenter still works (1e).
       - Phase 2: Task Force Archetype -- Design Time -- DB migration for task_mission_briefs and task_agent_roster,
       task force tools (set_task, add_agent, etc.), archetype block, WS events.
       - Phase 3: Belief Capture Archetype -- Design Time -- DB migration for belief_extraction_plans and beliefs,
       belief capture design-time tools, archetype block.
       - Phase 4: Room Archetype -- Design Time Tools -- Room configuration tools (set_meeting_purpose, add_member,
       etc.), room archetype block.
       - Phase 5: Task Force -- Runtime Execution -- DAG executor branch for task_force steps, agent prompts at
       runtime, sequential agent execution.
       - Phase 6: Belief Capture -- Runtime Execution -- Content normalization, gatekeeper module, belief extraction
       and storage.
       - Phase 7: Belief Injection into Rooms -- Inject upstream beliefs into room agent system prompts.
       - Phase 8: Mask Agent (Conversational Interface) -- Post-execution chat with mask agent that answers from
       beliefs.
       - Phase 9: Frontend -- Blank Nodes + Step Chat -- Blank node component, step chat panel, real-time WS updates,
       archetype-specific node skins.
       - Phase 10: Resource Nodes (Future) -- GitHub, Database, S3 resource nodes. Marked as future.
       - Phase 11: Runtime Planner (Future) -- Planner LLM call before task force execution. Marked as future.

       Completion status markers: No checkboxes or "done" markers anywhere. No status indicators at all. The document
       is purely a plan.

       Backend work: Phases 1-8 are all backend. Includes DB migrations, Rust modules (prompt system, tools, DAG
       executor branches, WS events, API endpoints), protocol file creation.

       Frontend work: Phase 9 covers frontend (blank nodes, chat panel, WS event handling, archetype-specific node
       skins). Phases 10-11 also have frontend implications but are marked "Future."

       Status: NOT STARTED as a formal tracking document, but the git diff and other tickets suggest Phase 1 and Phase
        2 work has been partially implemented on the feat/dynamic-nodes branch.

       ---
       2. /Users/davidcouch/Dev/gh-agents/tickets/new-node-system/BOCA_INTEGRATION_PLAN.md

       High-level purpose: Detailed design for the Belief Capture Protocol and Masked Conversations -- how to extract
       structured beliefs from upstream protocol outputs and use them in rooms and standalone chat. This is the
       deep-dive companion to Phases 3, 6, 7, and 8 of the full implementation plan.

       Key sections/phases:

       - Part 1: Belief Capture Protocol -- Protocol structure (mirrors documenter pattern with 3 phases: plan,
       extract, store), extraction plan schema, content normalization layer, gatekeeper prompt, migration
       0025_beliefs.sql, DB layer (BeliefRow, BeliefExtractionPlanRow, BeliefRepo trait), DAG integration.
       - Part 2: Masked Conversations -- Accumulate-never-remove belief retrieval, retrieval function using cheap LLM
       call (Haiku), session belief tracking (mask_session_beliefs table), room mask injection, standalone mask chat,
       belief formatting utility.
       - Part 3: API & Events -- Belief CRUD endpoints, extraction plan API, BeliefsExtracted WS event.
       - Implementation Order -- v1: Foundation + Fixed Gatekeeper (9 steps), v2: Curated Plans + Query Retrieval (6
       steps), v3: Frontend (5 steps).
       - Design Decisions -- Why protocol node, why read artifacts, why static labeling, why LLM queries, why
       assistant-driven config.
       - Future Enhancements -- report_finding tool, convergence protocol, per-agent belief filtering, cross-workflow
       mask, etc.

       Completion status markers: None. No checkboxes or done markers. The implementation order sections are numbered
       but not checked off.

       Backend work: Extensive -- DB migration, DB layer (row types, traits, PgRepo impl), belief capture executor,
       content normalization, gatekeeper module, belief retrieval, room mask injection, mask chat endpoint, API
       endpoints, WS events. Lists 7 files to create and 9 to modify.

       Frontend work: v3 section describes 5 frontend items: belief capture node on canvas, extraction plan assistant
       UI, beliefs panel, "Ask about this workflow" button, room meeting with belief retrieval indicator.

       Status: NOT STARTED. This is purely a design document with no implementation markers.

       ---
       3. /Users/davidcouch/Dev/gh-agents/tickets/new-node-system/GENERALIZED_AGENT_DESIGNER_TICKET.md

       High-level purpose: Extract the Agent Designer from being task-force-specific into a shared pre-lifecycle
       module that any archetype (task force, documenter, room) can use. Also explicitly replaces Phase 7 (Belief
       Injection into Rooms) from the full plan -- beliefs flow through the designer rather than being manually
       concatenated.

       Key sections/phases:

       - Part 1: Extract Shared Agent Designer Module -- Create src/server/hub/agent_designer/mod.rs with generic
       run_agent_designer(), input.rs (DesignerInput, AgentDefinition, UpstreamContext, ToolDescription), output.rs
       (DesignerResult, DesignedAgentPrompt), strategy.rs, tests.rs.
       - Part 2: Archetype Input Formatters -- Task force formatter (designer_input.rs), documenter formatter
       (strategist + research/write), room formatter (with belief injection), shared upstream formatter.
       - Part 3: Documenter Integration -- Replace static one-line templates with designer-generated prompts. Two
       designer calls per execution (strategist, then researchers + writers). Includes fallback to static templates on
        failure.
       - Part 4: Room Integration + Belief Flow (Replaces Phase 7) -- Designer call happens once before room loop.
       Beliefs flow through as additional_context on each AgentDefinition. Designer curates which beliefs each member
       sees based on perspective.
       - Part 5: Update DB Schema -- Generalize agent_designer_runs and agent_designer_outputs tables from
       task-force-specific to archetype-agnostic (adds archetype, phase, source_entity_id fields).
       - Part 6: Update Agent Designer Protocol Prompt -- Make the designer's own prompt archetype-agnostic.
       - Part 7: Testing -- Shared module tests, documenter designer input tests, room designer input tests,
       integration tests.
       - Appendices A-D -- Designer call count per archetype, Phase 7 replacement mapping, example room with belief
       curation, implementation order.

       Completion status markers: None. No checkboxes or done markers.

       Backend work: All backend. Creates shared agent_designer module, archetype-specific input formatters, modifies
       documenter phases, modifies room executor, updates DB schema, updates protocol prompt.

       Frontend work: None described in this ticket.

       Dependency: Explicitly depends on Phase 6 (Belief Capture Runtime) landing first.

       Status: NOT STARTED. Design document only. However, some of the Agent Designer work described in the original
       ticket (file 4 below) has been done, which this ticket would refactor.

       ---
       4. /Users/davidcouch/Dev/gh-agents/tickets/done/AGENT_DESIGNER_TICKET.md

       High-level purpose: The original Agent Designer ticket -- a pre-lifecycle function inside task force protocol
       execution that generates optimized (system_prompt, task_prompt) pairs for each agent using an LLM call. This is
        the task-force-specific version that the generalized ticket (file 3) would later extract and expand.

       Key sections/phases:

       - Part 1: Agent Designer Protocol Files -- config.yaml, system.md (with 21 BOCA-style beliefs baked in),
       prompt.md, register in protocols.rs.
       - Part 2: DB Schema for Designer Runs -- Migration for agent_designer_runs and agent_designer_outputs tables
       (task-force-specific with mission_brief_id, agent_roster_entry_id).
       - Part 3: Agent Designer Execution Function -- run_agent_designer() function in
       src/server/hub/dag/task_force/designer.rs, AgentDesignerStrategy.
       - Part 4: Task Force Execution Integration -- execute_task_force_step(), DAG executor routing for "task_force".
       - Part 5: Testing -- Unit tests (formatting, parsing, template resolution), integration tests (mock LLM, full
       pipeline, tool validation, previous outputs).
       - Appendix A -- The 21 beliefs with tags, confidence, sources.
       - Appendix B -- Example Agent Designer output (Scanner/Analyzer/Reporter).
       - Appendix C -- Prompt caching consideration.
       - Appendix D -- Future Haiku crew optimization.

       Completion status markers: This file is in tickets/done/, indicating it is considered DONE. The git status
       shows it was deleted from tickets/new-node-system/ and moved to tickets/done/.

       Backend work: All backend -- protocol files, DB migration, designer execution function, task force executor,
       DAG routing.

       Frontend work: None.

       Status: DONE. Moved to tickets/done/. The git diff shows active changes in
       src/server/hub/dag/agent_designer/tests.rs, src/server/hub/dag/designer_input/mod.rs,
       src/server/hub/dag/designer_input/room.rs, src/server/hub/dag/designer_input/tests.rs, confirming
       implementation has occurred.

       ---
       5. /Users/davidcouch/Dev/gh-agents/tickets/done/DYNAMIC_TASK_NODE_DESIGN.md

       High-level purpose: The foundational vision document for the entire dynamic task node system. Describes how
       every canvas node starts blank and gets configured through conversation. Covers the three layers
       (Assistant/Design Time, Planner/Runtime, Agents/Runtime), resource nodes, protocol nodes, the mission brief
       data model, and diverse example scenarios.

       Key sections/phases:

       - Canvas Node Types -- Resource nodes (static, user-configured), task nodes (dynamic, assistant-designed),
       protocol nodes (specialized behavior like belief capture and room).
       - Node Lifecycle -- blank -> configuring -> configured -> executed.
       - The Three Layers -- Layer 1: Assistant (design time, base + archetype block pattern), Layer 2: Planner
       (runtime, one LLM call), Layer 3: Agents (runtime, execute with generated prompts).
       - Resource Node -> Task Node: Capability Propagation -- Docker container provisioning, multiple resource
       composition.
       - Editing Configured Nodes -- Reopen chat, modify config, switch archetypes.
       - Cross-Node Awareness -- Assistant has graph-wide context.
       - Data Model -- task_mission_briefs, task_agent_roster, task_execution_plans tables.
       - Integration with Belief System -- Content normalization for task nodes.
       - Assistant Toolset -- Universal tools, per-archetype tools.
       - Execution Flow -- End-to-end example.
       - Example Scenarios -- 8 diverse examples (startup pitch deck, due diligence, movie screenplay, marketing
       campaign, academic research paper, incident response, event planning, hiring pipeline).
       - Build Path -- v1 (Mission Brief + Simple Execution), v2 (Runtime Planner), v3 (Workflow Assistant), v4
       (Resource Node Types).

       Completion status markers: This file is in tickets/done/, indicating it is considered DONE as a design
       document.

       Backend work: Describes the overall system architecture -- DB schema, executor, planner, container management.
       The implementation details live in the other tickets.

       Frontend work: Describes blank node UX, chat panel, real-time node updates, archetype switching. Implementation
        details live in Phase 9 of the full plan.

       Status: DONE as a design/vision document. Moved to tickets/done/. The actual implementation of the features
       described here spans across the other tickets.

       ---
       6. /Users/davidcouch/Dev/gh-agents/tickets/done/EMBEDDED_USER_CONTEXT_TICKET.md

       High-level purpose: Restructure the node assistant's design-time chat so that project state (graph context,
       current config) lives in the user message, not the system prompt. The system prompt stays behavioral (identity,
        tools, guidelines) and becomes stable/cacheable. The user message carries the current state rendered as a
       project briefing with the user's actual input appended at the bottom.

       Key sections/phases:

       - Part 1: User Context Templates -- Create per-archetype user context templates (base/user_context.md,
       documenter/user_context.md, task_force/user_context.md, belief_capture/user_context.md, room/user_context.md).
       Each uses {{.Context.*}} variables with {{.User.input}} at the bottom.
       - Part 2: Modify System Prompt Template -- Remove {{.System.graph_context}} and {{.System.current_config}} from
        system prompt. Add note that user provides state in messages.
       - Part 3: Rust -- Split Prompt Building -- Register user context templates in protocols.rs, modify
       build_step_system_prompt() to build_step_prompts() returning both system prompt and user context template,
       create build_user_context() function with per-archetype builders.
       - Part 4: ChatStrategy Integration -- Add user_context_template to ChatConfig, modify run_step_chat() to render
        user context before passing to strategy, chat message storage stores raw user input (not rendered context).
       - Part 5: Testing -- Template tests, user context rendering tests, system prompt exclusion tests, integration
       tests.
       - Appendix A -- Before/After comparison showing the split.
       - Appendix B -- Prompt caching impact analysis (~500-600 token stable system prompt vs ~800-1200 variable).

       Completion status markers: This file is in tickets/done/, indicating it is considered DONE.

       Backend work: All backend -- template files, Rust prompt building changes, ChatStrategy integration.

       Frontend work: None directly, though the rendered user context is transparent to the frontend (it just sends
       raw messages).

       Status: DONE. Moved to tickets/done/. The git diff shows changes in src/server/hub/dag/designer_input/mod.rs
       and related files, consistent with this pattern being implemented.

       ---
       7. /Users/davidcouch/Dev/gh-agents/tickets/done/NODE_ASSISTANT_PROMPTS.md

       High-level purpose: The complete prompt architecture for the node configuration assistant. Provides the actual
       prompt text for the base prompt and all four archetype blocks (documenter, task force, belief capture, room),
       including few-shot examples for each.

       Key sections/phases:

       - How It Works -- Base prompt loaded for blank nodes, archetype block swapped in on selection, user can change
       direction.
       - Base Prompt (~350 tokens) -- Identity, graph context injection point, archetype descriptions, guidelines.
       - Archetype: Documenter (~450 tokens) -- Three-phase pipeline context, tools (add_document_def,
       update_document_def, remove_document_def, set_research_scope, set_writing_style), two few-shot examples.
       - Archetype: Task Force (~500 tokens) -- Mission + roster context, tools (set_task, add_agent, update_agent,
       remove_agent, set_capabilities, set_failure_mode), two few-shot examples.
       - Archetype: Belief Capture (~400 tokens) -- Extraction plan context, tools (set_extraction_focus,
       set_tag_vocabulary, set_contradiction_handling, set_confidence_threshold), two few-shot examples.
       - Archetype: Room (~450 tokens) -- Meeting space context, tools (set_meeting_purpose, add_member,
       update_member, remove_member, set_max_turns, set_interaction_mode), two few-shot examples.
       - Token Budget Summary -- Shows per-archetype system prompt sizes.
       - Archetype Switching -- How switching works (swap block, preserve chat history, update tools).

       Completion status markers: This file is in tickets/done/, indicating it is considered DONE as a design
       document.

       Backend work: Provides the prompt content that gets implemented in protocol files and protocols.rs. Referenced
       by the full implementation plan.

       Frontend work: None.

       Status: DONE as a design/reference document. Moved to tickets/done/.

       ---
       Summary: Done vs. Remaining

       DONE (in tickets/done/):


















       File: DYNAMIC_TASK_NODE_DESIGN.md
       What It Covers: Overall vision/architecture
       Nature: Design document
       ────────────────────────────────────────
       File: NODE_ASSISTANT_PROMPTS.md
       What It Covers: Prompt text for all archetypes
       Nature: Reference document
       ────────────────────────────────────────
       File: AGENT_DESIGNER_TICKET.md
       What It Covers: Task-force-specific Agent Designer
       Nature: Implementation ticket (code written)
       ────────────────────────────────────────
       File: EMBEDDED_USER_CONTEXT_TICKET.md
       What It Covers: Move project state from system to user message
       Nature: Implementation ticket (code written)
       REMAINING (in tickets/new-node-system/):
       File: FULL_IMPLEMENTATION_PLAN.md
       What It Covers: Master roadmap, Phases 1-11
       Nature: Tracking document -- no phase has completion markers
       ────────────────────────────────────────
       File: BOCA_INTEGRATION_PLAN.md
       What It Covers: Belief capture + masked conversations
       Nature: Design/implementation plan -- not started
       ────────────────────────────────────────
       File: GENERALIZED_AGENT_DESIGNER_TICKET.md
       What It Covers: Extract Agent Designer to shared module for all archetypes
       Nature: Design/implementation plan -- not started, depends on Phase 6
       What is actively in-progress (based on git status):

       The feat/dynamic-nodes branch shows modifications to files consistent with Phase 1 (Generalize Archetype
       System) and parts of Phase 2 (Task Force Design Time) work, plus the Agent Designer and Designer Input
       implementations. Key modified files include:
       - src/server/hub/dag/designer_input/mod.rs and room.rs (designer input formatting)
       - src/server/hub/dag/agent_designer/tests.rs (agent designer tests)
       - src/server/hub/dag/room_step/mod.rs and tests.rs (room step execution)
       - src/server/hub/dag/documenter/mod.rs (documenter)
       - src/server/hub/dag/for_each/mod.rs (for-each)
       - src/server/hub/dag/utils/prompts.rs (prompt utilities)
       - Various server, config, and API files

       What is NOT started:

       - Phase 3 (Belief Capture Design Time)
       - Phase 4 (Room Design Time Tools)
       - Phase 5 (Task Force Runtime) -- partially addressed by Agent Designer ticket
       - Phase 6 (Belief Capture Runtime)
       - Phase 7 (Belief Injection into Rooms) -- to be replaced by Generalized Agent Designer
       - Phase 8 (Mask Agent)
       - Phase 9 (Frontend)
       - Phases 10-11 (Future)
       - BOCA Integration Plan (full implementation)
       - Generalized Agent Designer (depends on Phase 6)