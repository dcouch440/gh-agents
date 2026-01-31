# Ticket: Create Architecture Documentation

**Title:** Create `doc/architecture.md` summarizing the nexor project structure

**Tier:** Worker

**Priority:** Low

**Constraints:**
- Max files to modify: 1
- Allowed paths: `doc/`

---

## Description

Create a comprehensive `doc/architecture.md` file that documents the nexor project architecture. The document should be useful for onboarding new contributors and as a quick reference for existing developers.

## Requirements

1. **Read the following files for context:**
   - `CLAUDE.md` (project overview, conventions, source layout)
   - `PROGRESS.md` (current status)
   - `src/lib.rs` (module structure)
   - `src/main.rs` (entry point)
   - `Cargo.toml` (dependencies)

2. **Document these sections:**
   - **Overview**: What nexor is, in 2-3 sentences
   - **System Architecture**: High-level diagram (ASCII) showing Rust backend, React UI, Ink CLI, PostgreSQL
   - **Backend Modules**: Brief description of each `src/` subdirectory (types, config, db, llm, agents, orchestration, prompts, execution, github)
   - **Agent System**: How agents are structured (tiers, personas, execution loop, tool use)
   - **Orchestration**: How tasks flow from user input through planner → router → dispatcher → agent
   - **Frontend**: UI stack (React 19, Vite, Tailwind, Zustand) and key pages
   - **Data Flow**: How a chat message travels from the UI through the backend to agent execution and back
   - **Key Dependencies**: Core crates (axum, tokio, sqlx, serde) and their roles

3. **Style:**
   - Use markdown headers, bullet points, and code blocks
   - Keep it concise — aim for 150-250 lines
   - No fluff, developer-focused

## Acceptance Criteria

- [ ] `doc/architecture.md` exists and is well-structured
- [ ] All major system components are documented
- [ ] Data flow section traces a request end-to-end
- [ ] File is under 300 lines

## Verification

```bash
# File exists and is non-empty
test -s doc/architecture.md && echo "PASS" || echo "FAIL"

# Check structure has key sections
grep -c "## " doc/architecture.md  # Should be >= 6 sections
```
