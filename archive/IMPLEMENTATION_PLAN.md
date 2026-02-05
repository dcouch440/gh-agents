# Router Modes System - Implementation Plan

**Reference**: See `ROUTER_MODES_DESIGN.md` for full design details.

**Status**: 🔴 Not Started

---

## Phase 1: Database Foundation ⏸️

**Goal**: Create database schema for router modes system.

**Files**:
- `migrations/064_tool_router_modes.sql` (NEW)

**Tasks**:
- [ ] 1.1. Create migration file `064_tool_router_modes.sql`
- [ ] 1.2. Add `parent_router_id` and `level` to `tool_routers` table
- [ ] 1.3. Create `tool_router_modes` table (modes for each router)
  - Include `append_to_agent_system_prompt` BOOLEAN column (default FALSE) for append vs replace system prompt
  - Include `append_to_agent_tools` BOOLEAN column (default TRUE) for union vs replace tools
- [ ] 1.4. Create `tool_router_mode_tools` junction table (mode → tools)
- [ ] 1.5. Add `router_id` to `agents` table
- [ ] 1.6. Add `selected_router_mode_id` to `agent_executions` table
- [ ] 1.7. Add deprecation comment to `agent_modes` table
- [ ] 1.8. Create all necessary indexes
- [ ] 1.9. Run migration: `docker exec gh-agents-postgres-1 psql -U nexor -d nexor -f /migrations/064_tool_router_modes.sql`
- [ ] 1.10. Verify migration: `docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\dt tool_router*"`

**Verification**:
```bash
# Check tables exist
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "
  SELECT table_name FROM information_schema.tables
  WHERE table_name IN ('tool_router_modes', 'tool_router_mode_tools')
  ORDER BY table_name;
"

# Check constraints
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "
  SELECT constraint_name, constraint_type
  FROM information_schema.table_constraints
  WHERE table_name = 'tool_router_modes';
"
```

**Success Criteria**:
- ✅ Migration runs without errors
- ✅ All tables exist with correct columns
- ✅ All constraints and indexes created
- ✅ Can insert test data

---

## Phase 2: Database Layer (Rust) ⏸️

**Goal**: Implement Rust database layer for new tables.

**Files**:
- `src/db/mod.rs` (MODIFY)
- `src/db/traits/mod.rs` (MODIFY)
- `src/db/pg_repo/mod.rs` (MODIFY)
- `src/db/queries/mod.rs` (MODIFY)

**Tasks**:
- [ ] 2.1. Add `ToolRouterModeRow` struct to `src/db/mod.rs`
  ```rust
  pub struct ToolRouterModeRow {
      pub id: Uuid,
      pub router_id: Uuid,
      pub mode_key: String,
      pub display_name: String,
      pub description: String,
      pub system_prompt: String,
      pub temperature: f32,
      pub max_tokens: i32,
      pub append_to_agent_system_prompt: bool,  // NEW: Append vs replace system prompt
      pub append_to_agent_tools: bool,          // NEW: Union vs replace tools
      pub display_order: i32,
      pub created_at: DateTime<Utc>,
      pub updated_at: DateTime<Utc>,
  }
  ```
- [ ] 2.2. Add `ToolRouterModeRow` to module exports
- [ ] 2.3. Extend `ToolRouterRepo` trait in `src/db/traits/mod.rs`:
  - [ ] `list_router_modes(router_id) -> Vec<ToolRouterModeRow>`
  - [ ] `get_router_mode(id) -> Option<ToolRouterModeRow>`
  - [ ] `get_router_mode_by_key(router_id, key) -> Option<ToolRouterModeRow>`
  - [ ] `create_router_mode(...) -> ToolRouterModeRow`
  - [ ] `update_router_mode(...) -> ToolRouterModeRow`
  - [ ] `delete_router_mode(id)`
  - [ ] `get_mode_tools(mode_id) -> Vec<ToolRow>`
  - [ ] `set_mode_tools(mode_id, tool_ids)`
- [ ] 2.4. Implement trait methods in `src/db/pg_repo/mod.rs`
- [ ] 2.5. Add query functions to `src/db/queries/mod.rs`
- [ ] 2.6. Write unit tests for each repo method
- [ ] 2.7. Run tests: `cargo test db::pg_repo::tool_router`
- [ ] 2.8. Run `cargo check` to verify compilation

**Verification**:
```bash
# Compile check
cargo check

# Run database tests
cargo test db::pg_repo::tool_router -- --nocapture

# Test specific methods
cargo test list_router_modes
cargo test create_router_mode
cargo test get_mode_tools
```

**Success Criteria**:
- ✅ All trait methods implemented
- ✅ `cargo check` passes
- ✅ All unit tests pass
- ✅ Can create/read/update/delete modes via Rust code
- ✅ Can associate tools with modes

---

## Phase 3: Tool Registry ⏸️

**Goal**: Create static tool registry for mapping tool names to implementations.

**Files**:
- `src/tools/` (NEW directory)
  - `src/tools/mod.rs`
  - `src/tools/registry/mod.rs`
  - `src/tools/registry/tests.rs`
- `src/lib.rs` (MODIFY)

**Tasks**:
- [ ] 3.1. Create `src/tools/` directory
- [ ] 3.2. Create `src/tools/mod.rs` with module re-exports: `pub mod registry; pub use registry::*;`
- [ ] 3.3. Create `src/tools/registry/` subdirectory
- [ ] 3.4. Create `src/tools/registry/mod.rs`:
  - [ ] `get_tool_definition(name: &str) -> Option<Tool>`
  - [ ] Match statement for all existing tools
  - [ ] Helper functions for each tool definition
  - [ ] Add `mod tests;` at bottom to link tests.rs
- [ ] 3.5. Create `src/tools/registry/tests.rs` with test module
- [ ] 3.6. Add `pub mod tools;` to `src/lib.rs`
- [ ] 3.7. Document each tool with description and parameters
- [ ] 3.8. Write tests for registry
- [ ] 3.9. Run tests: `cargo test tools::registry`
- [ ] 3.10. Run `cargo check`

**Tool List** (to implement in registry):
- [ ] `bash` - Execute bash commands
- [ ] `read_file` - Read file contents
- [ ] `write_file` - Write file contents
- [ ] `edit_file` - Edit file with search/replace
- [ ] `search_code` - Search codebase
- [ ] `web_search` - Search the web (if exists)
- [ ] `github_*` - GitHub operations (if exist)
- [ ] ...add others as discovered

**Verification**:
```bash
# Check registry compiles
cargo check

# Test registry
cargo test tools::registry -- --nocapture

# Test specific tool
cargo test tools::registry::test_bash_tool
```

**Success Criteria**:
- ✅ Registry compiles without errors
- ✅ All existing tools mapped
- ✅ `get_tool_definition("bash")` returns valid Tool
- ✅ `get_tool_definition("invalid")` returns None
- ✅ Tests pass

---

## Phase 4: AgentOrchestrator ⏸️

**Goal**: Create unified execution layer that handles routing automatically.

**Files**:
- `src/server/hub/orchestrator/` (NEW directory)
  - `src/server/hub/orchestrator/mod.rs`
  - `src/server/hub/orchestrator/tests.rs`
- `src/server/hub/mod.rs` (MODIFY - add module)
- `src/server/state/mod.rs` (MODIFY - add orchestrator to AppState)

**Tasks**:
- [ ] 4.1. Create `src/server/hub/orchestrator/` directory:
  - [ ] Create `src/server/hub/orchestrator/mod.rs` with implementation
  - [ ] Create `src/server/hub/orchestrator/tests.rs` with test module
  - [ ] Add `mod tests;` at bottom of mod.rs to link tests
- [ ] 4.2. Define `AgentOrchestrator` struct in mod.rs:
  - [ ] `engine: ExecutionEngine`
  - [ ] `repo: Arc<dyn ServerRepo>`
  - [ ] `router_repo: Arc<dyn ToolRouterRepo>`
- [ ] 4.3. Implement `AgentOrchestrator::new()`
- [ ] 4.4. Implement `execute_agent()` method:
  - [ ] Load agent from database
  - [ ] Check if agent has `router_id`
  - [ ] If yes: call `route_with_history()`
  - [ ] If no: use agent defaults
  - [ ] Create strategy with config
  - [ ] Call `engine.execute()`
  - [ ] Return `OrchestratedResult`
- [ ] 4.5. Implement `route_with_history()` method:
  - [ ] Load router config
  - [ ] Load available modes
  - [ ] Build classification prompt WITH FULL HISTORY
  - [ ] Execute routing via RouterStrategy
  - [ ] Parse mode key from response
  - [ ] Load selected mode
  - [ ] **Handle `append_to_agent_system_prompt` flag**:
    - [ ] If TRUE: Concatenate agent's system_prompt + "\n\n" + mode's system_prompt
    - [ ] If FALSE: Use ONLY mode's system_prompt (replace agent's)
  - [ ] Load mode tools from DB
  - [ ] **Handle `append_to_agent_tools` flag**:
    - [ ] If TRUE: Load agent's base tools + union with mode tools (deduplicate by name)
    - [ ] If FALSE: Use ONLY mode tools (replace/ignore agent's base tools)
  - [ ] Map tools via registry
  - [ ] Return mode config
- [ ] 4.6. Implement `default_config_for_agent()` method
- [ ] 4.7. Add `OrchestratedResult` struct
- [ ] 4.8. Add `OrchestratorError` enum
- [ ] 4.9. Add orchestrator to `AppState`:
  - [ ] Add field `agent_orchestrator: AgentOrchestrator`
  - [ ] Initialize in `AppState::new()`
- [ ] 4.10. Add `pub mod orchestrator;` to `src/server/hub/mod.rs`
- [ ] 4.11. Write unit tests
- [ ] 4.12. Run tests: `cargo test hub::orchestrator`
- [ ] 4.13. Run `cargo check`

**Key Implementation Details**:
```rust
// Classification prompt must include history
fn build_classification_prompt(
    input: &str,
    history: &[Message],
    modes: &[ToolRouterModeRow]
) -> String {
    format!(
        "## Conversation History:\n{}\n\n\
         ## Current Input:\n{}\n\n\
         ## Available Modes:\n{}\n\n\
         Output ONLY the mode key.",
        format_history(history, 10),
        input,
        format_modes(modes)
    )
}

// Construct system prompt based on append_to_agent_system_prompt flag
fn build_system_prompt(
    agent: &AgentRow,
    mode: &ToolRouterModeRow,
) -> String {
    if mode.append_to_agent_system_prompt {
        // APPEND: Combine agent's prompt + mode's prompt
        format!("{}\n\n{}", agent.system_prompt, mode.system_prompt)
    } else {
        // REPLACE: Use only mode's prompt
        mode.system_prompt.clone()
    }
}

// Union vs replace tools based on append_to_agent_tools flag
async fn load_final_tools(
    &self,
    agent_id: Uuid,
    mode: &ToolRouterModeRow,
) -> Result<Vec<Tool>, OrchestratorError> {
    // Load mode's tools
    let mode_tool_ids = self.router_repo.get_mode_tools(mode.id).await?;
    let mode_tool_rows = self.repo.get_tools_by_ids(&mode_tool_ids).await?;

    let final_tool_rows = if mode.append_to_agent_tools {
        // UNION: Combine agent's base tools + mode tools
        let agent_tool_ids = self.repo.get_agent_tools(agent_id).await?;
        let agent_tool_rows = self.repo.get_tools_by_ids(&agent_tool_ids).await?;

        // Deduplicate by tool name (mode tools take precedence)
        union_tools_by_name(agent_tool_rows, mode_tool_rows)
    } else {
        // REPLACE: Use only mode tools
        mode_tool_rows
    };

    // Map to Tool definitions via registry
    let tools = final_tool_rows
        .iter()
        .filter_map(|row| registry::get_tool_definition(&row.name))
        .collect();

    Ok(tools)
}

fn union_tools_by_name(agent_tools: Vec<ToolRow>, mode_tools: Vec<ToolRow>) -> Vec<ToolRow> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    // Add mode tools first (take precedence)
    for tool in mode_tools {
        seen.insert(tool.name.clone());
        result.push(tool);
    }

    // Add agent tools that aren't already in mode tools
    for tool in agent_tools {
        if !seen.contains(&tool.name) {
            result.push(tool);
        }
    }

    result
}
```

**Verification**:
```bash
# Compile check
cargo check

# Run orchestrator tests
cargo test hub::orchestrator -- --nocapture

# Test routing logic
cargo test route_with_history
cargo test execute_agent
```

**Success Criteria**:
- ✅ AgentOrchestrator compiles
- ✅ Can execute agent without router (fallback works)
- ✅ Can execute agent with router (routing works)
- ✅ Router sees full conversation history
- ✅ Tools are filtered per mode
- ✅ Returns result with mode metadata
- ✅ All tests pass

---

## Phase 5: Update Call Sites ⏸️

**Goal**: Replace all agent execution calls with orchestrator.

**Files**:
- `src/server/hub/mod.rs` (MODIFY `run_chat`)
- `src/server/chat_consumer/mod.rs` (MODIFY if needed)
- `src/server/room_executor/mod.rs` (MODIFY)
- `src/server/workflow_executor/mod.rs` (MODIFY if exists)
- Any other files that execute agents

**Tasks**:
- [ ] 5.1. Find all call sites:
  ```bash
  grep -r "ExecutionEngine\|run_chat\|ChatStrategy" src/server --include="*.rs"
  ```
- [ ] 5.2. Update `run_chat()` in `src/server/hub/mod.rs`:
  - [ ] Remove manual agent loading
  - [ ] Remove manual tool loading
  - [ ] Remove old agent_modes logic
  - [ ] Call `state.agent_orchestrator.execute_agent()`
  - [ ] Save result with `selected_router_mode_id`
- [ ] 5.3. Update `chat_consumer.rs` (if needed)
- [ ] 5.4. Update `room_executor.rs`:
  - [ ] Replace agent execution with orchestrator
  - [ ] Pass full room history to orchestrator
- [ ] 5.5. Update `workflow_executor.rs` (if exists):
  - [ ] Replace agent execution with orchestrator
  - [ ] Pass workflow history to orchestrator
- [ ] 5.6. Search for any other execution points
- [ ] 5.7. Remove old routing/agent_modes code that's now obsolete
- [ ] 5.8. Run `cargo check`
- [ ] 5.9. Run integration tests
- [ ] 5.10. Test manually:
  - [ ] Send chat message (should work)
  - [ ] Execute workflow (should work)
  - [ ] Run room session (should work)

**Before/After Example**:
```rust
// ❌ BEFORE (in run_chat)
let agent = state.repo.get_agent(agent_id).await?;
let tools = state.repo.get_agent_tools(agent_id).await?;
let modes = state.repo.get_agent_modes(agent_id).await?;
// ... complex routing logic ...
let strategy = ChatStrategy::new(config);
let engine = ExecutionEngine::new(provider);
let result = engine.execute(&strategy, ...).await?;

// ✅ AFTER
let result = state.agent_orchestrator.execute_agent(
    agent_id,
    input,
    history,  // Full conversation history!
    &sink,
    &recorder,
    cancel
).await?;

// Save with routing metadata
db.insert_agent_execution(AgentExecutionRow {
    selected_router_mode_id: result.selected_mode_id,
    ...
});
```

**Verification**:
```bash
# Compile check
cargo check

# Run all tests
cargo test

# Manual testing
cargo run
# Then test:
# - Send chat message via API
# - Check logs for routing
# - Verify tools are filtered
```

**Success Criteria**:
- ✅ `cargo check` passes
- ✅ All call sites updated
- ✅ Old routing code removed
- ✅ Chat messages work
- ✅ Workflows work (if applicable)
- ✅ Rooms work (if applicable)
- ✅ Routing metadata saved to DB

---

## Phase 6: API Endpoints ⏸️

**Goal**: Create REST API for managing router modes.

**Files**:
- `src/server/api/router_modes/mod.rs` (NEW)
- `src/server/api/router_modes/tests.rs` (NEW)
- `src/server/api/mod.rs` (MODIFY - add re-exports)
- `src/server/mod.rs` (MODIFY - register routes)
- `src/constants.rs` (MODIFY - add route constants)

**Tasks**:
- [ ] 6.1. Create `src/server/api/router_modes/` directory
- [ ] 6.2. Create `mod.rs` with handler functions:
  - [ ] `list_router_modes(router_id)` - GET /routers/:id/modes
  - [ ] `get_router_mode(router_id, mode_id)` - GET /routers/:id/modes/:mid
  - [ ] `create_router_mode(router_id, request)` - POST /routers/:id/modes
  - [ ] `update_router_mode(router_id, mode_id, request)` - PATCH /routers/:id/modes/:mid
  - [ ] `delete_router_mode(router_id, mode_id)` - DELETE /routers/:id/modes/:mid
  - [ ] `get_mode_tools(router_id, mode_id)` - GET /routers/:id/modes/:mid/tools
  - [ ] `set_mode_tools(router_id, mode_id, request)` - PUT /routers/:id/modes/:mid/tools
- [ ] 6.3. Create request/response types
- [ ] 6.4. Add route constants to `src/constants.rs`
- [ ] 6.5. Register routes in `src/server/mod.rs`
- [ ] 6.6. Add re-exports to `src/server/api/mod.rs`
- [ ] 6.7. Create `tests.rs` with integration tests
- [ ] 6.8. Run tests: `cargo test api::router_modes`
- [ ] 6.9. Run `cargo check`
- [ ] 6.10. Test with curl/Postman:
  ```bash
  # Create mode
  curl -X POST http://localhost:3000/api/tool-routers/$ROUTER_ID/modes \
    -H "Content-Type: application/json" \
    -d '{"mode_key": "coding", "display_name": "Coding Mode", ...}'

  # List modes
  curl http://localhost:3000/api/tool-routers/$ROUTER_ID/modes

  # Set tools for mode
  curl -X PUT http://localhost:3000/api/tool-routers/$ROUTER_ID/modes/$MODE_ID/tools \
    -H "Content-Type: application/json" \
    -d '{"tool_ids": ["uuid1", "uuid2", "uuid3"]}'
  ```

**Verification**:
```bash
# Run API tests
cargo test api::router_modes -- --nocapture

# Start server
cargo run

# Test endpoints (in another terminal)
./test_api.sh
```

**Success Criteria**:
- ✅ All endpoints compile
- ✅ Can create/read/update/delete modes
- ✅ Can set tools for modes
- ✅ Proper error handling (404, 400, etc.)
- ✅ All tests pass
- ✅ Manual API testing works

---

## Phase 7: Frontend Types & API Client ⏸️

**Goal**: Add TypeScript types and API client for router modes.

**Files**:
- `frontend/src/types/router.ts` (NEW)
- `frontend/src/api/api.ts` (MODIFY)

**Tasks**:
- [ ] 7.1. Create `frontend/src/types/router.ts`:
  - [ ] `RouterMode` type (include `append_to_agent_system_prompt: boolean` and `append_to_agent_tools: boolean`)
  - [ ] `CreateRouterModeRequest` type (include `append_to_agent_system_prompt?: boolean` and `append_to_agent_tools?: boolean`)
  - [ ] `UpdateRouterModeRequest` type (include `append_to_agent_system_prompt?: boolean` and `append_to_agent_tools?: boolean`)
  - [ ] `SetModeToolsRequest` type
- [ ] 7.2. Add router modes endpoints to `api.ts`:
  - [ ] `routerModes.list(routerId)`
  - [ ] `routerModes.get(routerId, modeId)`
  - [ ] `routerModes.create(routerId, request)`
  - [ ] `routerModes.update(routerId, modeId, request)`
  - [ ] `routerModes.delete(routerId, modeId)`
  - [ ] `routerModes.getTools(routerId, modeId)`
  - [ ] `routerModes.setTools(routerId, modeId, request)`
- [ ] 7.3. Run TypeScript check: `npx tsc --noEmit`
- [ ] 7.4. Run ESLint: `npx eslint .`
- [ ] 7.5. Fix any errors

**Verification**:
```bash
cd frontend

# Type check
npx tsc --noEmit

# Lint
npx eslint .

# Build
npx vite build
```

**Success Criteria**:
- ✅ Types compile without errors
- ✅ API client types match backend
- ✅ ESLint passes (zero warnings)
- ✅ Frontend builds successfully

---

## Phase 8: Frontend UI ⏸️

**Goal**: Build UI for configuring router modes.

**Files**:
- `frontend/src/pages/Routers/RouterModesPage.tsx` (NEW)
- `frontend/src/pages/Routers/components/ModesList.tsx` (NEW)
- `frontend/src/pages/Routers/components/ModeEditor.tsx` (NEW)
- `frontend/src/pages/Routers/components/ModeToolSelector.tsx` (NEW)
- `frontend/src/App.tsx` (MODIFY - add route)
- `frontend/src/constants.ts` (MODIFY - add routes)

**Tasks**:
- [ ] 8.1. Create `frontend/src/pages/Routers/` directory
- [ ] 8.2. Create `RouterModesPage.tsx`:
  - [ ] List all modes for a router
  - [ ] Add "Create Mode" button
  - [ ] Edit/Delete actions per mode
- [ ] 8.3. Create `ModesList.tsx`:
  - [ ] Display modes in table/cards
  - [ ] Show mode_key, display_name, description
  - [ ] Show tool count
  - [ ] Actions (edit, delete)
- [ ] 8.4. Create `ModeEditor.tsx` (modal):
  - [ ] Form fields: mode_key, display_name, description
  - [ ] Form fields: system_prompt (textarea)
  - [ ] Form fields: temperature, max_tokens
  - [ ] **Checkbox: "Append to agent's system prompt" (append_to_agent_system_prompt)**
    - [ ] Default unchecked (FALSE)
    - [ ] Tooltip: "When checked, mode's system prompt is appended to agent's base prompt. When unchecked, mode's prompt completely replaces agent's prompt."
  - [ ] **Checkbox: "Add to agent's base tools" (append_to_agent_tools)**
    - [ ] Default checked (TRUE)
    - [ ] Tooltip: "When checked, mode tools are added to agent's tools. When unchecked, mode tools replace agent's tools."
  - [ ] Validation (mode_key must be snake_case)
  - [ ] Save/Cancel buttons
- [ ] 8.5. Create `ModeToolSelector.tsx`:
  - [ ] Load all available tools
  - [ ] Multi-select interface
  - [ ] Show selected tools
  - [ ] Save button
- [ ] 8.6. Add route to `App.tsx`
- [ ] 8.7. Add route constants to `constants.ts`
- [ ] 8.8. Test UI manually:
  - [ ] Create new mode
  - [ ] Edit mode
  - [ ] Delete mode
  - [ ] Set tools for mode
- [ ] 8.9. Run type check: `npx tsc --noEmit`
- [ ] 8.10. Run lint: `npx eslint .`

**UI Flow**:
```
Router Details Page
  ├─ Router Info (name, description)
  ├─ Modes Section
  │   ├─ [+ Create Mode] button
  │   ├─ Mode Card: "Coding"
  │   │   ├─ Description: "For programming tasks"
  │   │   ├─ Tools: bash, read_file, edit_file, search_code (4)
  │   │   ├─ Temp: 0.3 | Max Tokens: 8000
  │   │   └─ [Edit] [Delete] [Configure Tools]
  │   ├─ Mode Card: "Research"
  │   └─ Mode Card: "Chat"
  └─ [Save Router]

Mode Editor Modal
  ├─ Mode Key: [coding________] (snake_case, auto-validated)
  ├─ Display Name: [Coding Mode_____]
  ├─ Description: [For programming tasks...]
  ├─ System Prompt: [You are an expert programmer...]
  ├─ Temperature: [0.3___] (0.0 - 2.0)
  ├─ Max Tokens: [8000__]
  └─ [Save] [Cancel]

Tool Selector Modal
  ├─ Available Tools (checkbox list)
  │   ☑ bash
  │   ☑ read_file
  │   ☑ edit_file
  │   ☑ search_code
  │   ☐ web_search
  │   ☐ github_create_pr
  └─ [Save Selection]
```

**Verification**:
```bash
cd frontend

# Dev server
npm run dev

# Manual testing in browser:
# 1. Navigate to router details page
# 2. Create a new mode
# 3. Edit the mode
# 4. Configure tools for mode
# 5. Delete the mode

# Type check
npx tsc --noEmit

# Lint (must pass with zero warnings)
npx eslint .
```

**Success Criteria**:
- ✅ Can view all modes for a router
- ✅ Can create new mode with validation
- ✅ Can edit existing mode
- ✅ Can delete mode (with confirmation)
- ✅ Can select tools for mode
- ✅ Form validation works (mode_key format, required fields)
- ✅ TypeScript compiles
- ✅ ESLint passes (zero warnings)

---

## Phase 9: Testing ⏸️

**Goal**: Comprehensive testing of the entire system.

**Tasks**:
- [ ] 9.1. Backend Unit Tests:
  - [ ] Database layer tests
  - [ ] Tool registry tests
  - [ ] Orchestrator tests
  - [ ] API endpoint tests
- [ ] 9.2. Backend Integration Tests:
  - [ ] End-to-end routing flow
  - [ ] Agent execution with routing
  - [ ] Agent execution without routing (fallback)
  - [ ] Mode tool filtering
  - [ ] Error cases (no modes, invalid mode key, etc.)
- [ ] 9.3. Frontend Unit Tests:
  - [ ] Component tests (ModesList, ModeEditor, etc.)
  - [ ] Form validation tests
- [ ] 9.4. Frontend Integration Tests:
  - [ ] Create mode flow
  - [ ] Edit mode flow
  - [ ] Delete mode flow
  - [ ] Tool selection flow
- [ ] 9.5. Manual End-to-End Testing:
  - [ ] Create router in UI
  - [ ] Create 3 modes (coding, research, chat)
  - [ ] Configure tools for each mode
  - [ ] Assign router to agent
  - [ ] Send chat messages
  - [ ] Verify routing happens
  - [ ] Check database for selected_router_mode_id
  - [ ] Verify only filtered tools sent to LLM
- [ ] 9.6. Performance Testing:
  - [ ] Routing latency (should be <200ms for Haiku)
  - [ ] Memory usage with 50 modes
  - [ ] Concurrent routing requests

**Test Scenarios**:
```
Scenario 1: Agent with Router
  1. Create router "Emotional Router" with 3 modes
  2. Assign router to agent "Home"
  3. Send message: "I'm really struggling with this bug"
  4. Expected: Routes to "supportive" mode
  5. Verify: selected_router_mode_id in DB
  6. Verify: Only supportive mode's tools sent to LLM

Scenario 2: Agent without Router (Fallback)
  1. Agent has no router_id
  2. Send message
  3. Expected: Uses agent defaults
  4. Verify: selected_router_mode_id is NULL
  5. Verify: All agent tools sent to LLM

Scenario 3: Router with Full History
  1. Start conversation with "supportive" mode
  2. Send follow-up: "Actually, just give me the answer"
  3. Expected: Router sees full history, switches to "direct" mode
  4. Verify: Mode changed mid-conversation

Scenario 4: Invalid Mode Key
  1. Router returns invalid mode key
  2. Expected: Falls back to first mode (by display_order)
  3. Verify: Warning logged
  4. Verify: Execution succeeds with fallback

Scenario 5: Mode with No Tools
  1. Create "chat" mode with empty tools
  2. Expected: Works fine (pure conversation)
  3. Verify: LLM called with no tools

Scenario 6: Tool Not in Registry
  1. Mode has tool "unknown_tool" in DB
  2. Expected: Warning logged, tool skipped
  3. Verify: Other tools still work
```

**Verification**:
```bash
# Backend tests
cargo test

# Frontend tests
cd frontend && npm test

# Run specific test suites
cargo test orchestrator::tests::test_routing_with_history
cargo test api::router_modes::tests

# Integration test
cargo test --test integration_router_modes
```

**Success Criteria**:
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ Manual E2E scenarios work
- ✅ No performance regressions
- ✅ Error cases handled gracefully

---

## Phase 10: Migration from agent_modes ⏸️

**Goal**: Migrate existing agent_modes data to new system.

**Files**:
- `scripts/migrate_agent_modes.sql` (NEW)

**Tasks**:
- [ ] 10.1. Backup database:
  ```bash
  docker exec gh-agents-postgres-1 pg_dump -U nexor nexor > backup_pre_migration.sql
  ```
- [ ] 10.2. Analyze existing agent_modes data:
  ```sql
  SELECT COUNT(*) FROM agent_modes;
  SELECT agent_id, COUNT(*) FROM agent_modes GROUP BY agent_id;
  ```
- [ ] 10.3. Create migration script `scripts/migrate_agent_modes.sql`:
  - [ ] Create router for each agent that has modes
  - [ ] Link agents to routers
  - [ ] Migrate modes to tool_router_modes
  - [ ] Migrate tool_overrides (if possible)
- [ ] 10.4. Test migration on copy of DB
- [ ] 10.5. Run migration on production:
  ```bash
  docker exec gh-agents-postgres-1 psql -U nexor -d nexor -f /scripts/migrate_agent_modes.sql
  ```
- [ ] 10.6. Verify migration:
  ```sql
  -- Check router creation
  SELECT COUNT(*) FROM tool_routers WHERE name LIKE '%Agent%Router%';

  -- Check mode migration
  SELECT COUNT(*) FROM tool_router_modes;

  -- Check agent linking
  SELECT COUNT(*) FROM agents WHERE router_id IS NOT NULL;
  ```
- [ ] 10.7. Test agents with migrated data
- [ ] 10.8. Monitor for issues
- [ ] 10.9. After 1 week of stability, drop agent_modes:
  ```sql
  DROP TABLE agent_modes_versions CASCADE;
  DROP TABLE agent_modes CASCADE;
  ```

**Verification**:
```sql
-- Before migration
SELECT
  a.name AS agent,
  am.name AS mode,
  am.classifier_hint
FROM agents a
JOIN agent_modes am ON am.agent_id = a.id
ORDER BY a.name, am.name;

-- After migration
SELECT
  a.name AS agent,
  tr.name AS router,
  trm.mode_key,
  trm.display_name
FROM agents a
JOIN tool_routers tr ON tr.id = a.router_id
JOIN tool_router_modes trm ON trm.router_id = tr.id
ORDER BY a.name, trm.mode_key;
```

**Success Criteria**:
- ✅ All agent_modes data migrated
- ✅ Agents with modes now have router_id
- ✅ Existing functionality preserved
- ✅ No data loss
- ✅ agent_modes table dropped after verification

---

## Phase 11: Documentation & Deployment ⏸️

**Goal**: Document the system and deploy to production.

**Tasks**:
- [ ] 11.1. Update README.md with router modes section
- [ ] 11.2. Create user guide: `docs/ROUTER_MODES_GUIDE.md`
- [ ] 11.3. Create API documentation (OpenAPI/Swagger)
- [ ] 11.4. Add inline code documentation
- [ ] 11.5. Create video tutorial (optional)
- [ ] 11.6. Update CLAUDE.md with new conventions
- [ ] 11.7. Tag release: `git tag v1.0.0-router-modes`
- [ ] 11.8. Deploy to staging
- [ ] 11.9. Test on staging
- [ ] 11.10. Deploy to production
- [ ] 11.11. Monitor metrics:
  - [ ] Routing latency
  - [ ] Error rates
  - [ ] Token usage (should decrease)
  - [ ] User feedback

**Documentation Checklist**:
- [ ] Architecture diagram
- [ ] Database schema diagram (updated ERD)
- [ ] API endpoint documentation
- [ ] Frontend component documentation
- [ ] Example configurations
- [ ] Troubleshooting guide
- [ ] Performance optimization tips

**Success Criteria**:
- ✅ Complete documentation
- ✅ Successfully deployed
- ✅ No production incidents
- ✅ Metrics look good

---

## Rollback Plan 🚨

If something goes wrong:

1. **Database Rollback**:
   ```sql
   -- Restore from backup
   docker exec -i gh-agents-postgres-1 psql -U nexor nexor < backup_pre_migration.sql
   ```

2. **Code Rollback**:
   ```bash
   git revert <commit-range>
   cargo build
   ```

3. **Gradual Rollout**:
   - Deploy to 10% of agents first
   - Monitor for issues
   - Gradually increase to 100%

---

## Success Metrics 📊

Track these after deployment:

1. **Performance**:
   - Routing latency: <200ms (p95)
   - Overall response time: No regression
   - Token usage: 20-30% reduction

2. **Adoption**:
   - Number of routers created
   - Number of modes configured
   - Percentage of agents using routing

3. **Quality**:
   - Error rate: <1%
   - Mode selection accuracy: User feedback
   - Context optimization: Measure tool counts per request

4. **Cost**:
   - Input tokens per request: Reduced by 20-30%
   - Total API cost: Reduced by 15-25%

---

## Questions & Decisions Log 📝

| Question | Decision | Date | Rationale |
|----------|----------|------|-----------|
| Keep ExecutionEngine pure? | ✅ Yes | 2026-02-04 | Maintains single responsibility |
| Router sees full history? | ✅ Yes | 2026-02-04 | Better context for classification |
| Drop agent_modes? | ✅ Yes | 2026-02-04 | Redundant with new system |
| Tool-level routing? | ❌ No | 2026-02-04 | Too complex, YAGNI |
| Hierarchical routing (L1→L2→L3)? | ⏸️ Future | 2026-02-04 | Implement later if needed |

---

**Plan Version**: 1.0
**Created**: 2026-02-04
**Last Updated**: 2026-02-04
**Status**: 🔴 Ready to Begin
