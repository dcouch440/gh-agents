# Epic: Knowledge Rooms & Role Hierarchy

> A system for structured conversation, role-based context, and PRD-driven execution.

**Status:** Ideation
**Dependencies:** M3 (Agent Runtime), M4 (Prompts), M5 (Orchestration)
**Target:** Post-M5 (M10+)

---

## Summary

A markdown-based system where:
1. **Conversation rooms** capture fluid discussion
2. **PRD** crystallizes intent (the catalyst)
3. **Roles** define what agents read and how they interpret
4. **Hierarchy** is determined by file access

---

## The Core Model

```
┌─────────────────────────────────────────────────────────┐
│  💬 Conversation Room                                   │
│     - Fluid, exploratory                                │
│     - Target agnostic (code, design, docs, anything)    │
│     - No commitment until PRD                           │
└──────────────────────────┬──────────────────────────────┘
                           ▼
                  ┌────────────────┐
                  │      PRD       │  ← THE CATALYST
                  │                │
                  │  Crystallized  │
                  │     intent     │
                  └───────┬────────┘
                          ▼
┌─────────────────────────────────────────────────────────┐
│  Structured Execution                                   │
│                                                         │
│  📋 Decomposition → 🎫 Tickets → 💻 Implementation      │
└─────────────────────────────────────────────────────────┘
```

**Key Insight:** PRD is the phase transition.
- Before PRD: ideas can be abandoned, directions changed
- After PRD: committed, decomposable, trackable

---

## Concept 1: Roles as File Access

**Hierarchy IS file access.** Like video game classes, roles are defined by what documents they can/must read.

### Role Definition

```toml
[role.orchestrator]
category = "planning"
description = "Decomposes work into actionable tickets"
required_reading = ["PRD.md", "ROADMAP.md", "PROGRESS.md"]
can_delegate_to = ["worker", "utility"]
output_format = "plan"

[role.worker]
category = "implementation"
description = "Implements tickets according to spec"
required_reading = ["decomp/{ticket}.md", "CONVENTIONS.md"]
can_delegate_to = ["utility"]
output_format = "code + report"

[role.utility]
category = "support"
description = "Performs focused helper tasks"
required_reading = ["{task_specific}.md"]
can_delegate_to = []
output_format = "result"
```

### Predefined Role Library

Users select from categories:

```
📂 Analysis
   ├── Complaint Finder - "Extracts user frustrations"
   ├── Opportunity Spotter - "Identifies feature requests"
   └── Risk Assessor - "Finds potential failures"

📂 Planning
   ├── Scope Definer - "Breaks work into deliverables"
   ├── Prioritizer - "Ranks by impact and effort"
   └── Dependency Mapper - "Identifies blockers"

📂 Implementation
   ├── Architect - "Designs system structure"
   ├── Builder - "Writes code to spec"
   └── Reviewer - "Validates quality"

📂 Communication
   ├── Summarizer - "Condenses information"
   └── Formatter - "Structures output"
```

---

## Concept 2: Role as Interpretation Lens

Same document, different meaning based on role:

```
┌──────────────────────────────────────────────────────────┐
│  Human Feedback Room                                     │
│                                                          │
│  "Login takes forever, my competitor's app lets me in    │
│   with one tap. So frustrating."                         │
└──────────────────────────────────────────────────────────┘
              │
              │ Same text, different roles
              ▼
┌──────────────────┬──────────────────┬──────────────────┐
│ 😤 Complaint     │ 💡 Feature       │ 🔍 Competitive   │
│    Agent         │    Agent         │    Agent         │
├──────────────────┼──────────────────┼──────────────────┤
│ "frustration"    │ "one-tap auth"   │ "competitor X    │
│ "friction"       │ "biometric"      │  has faster      │
│ emotion: HIGH    │ priority: HIGH   │  login"          │
└──────────────────┴──────────────────┴──────────────────┘
```

The agent's job isn't to read literally - it's to:
1. Observe the room/document
2. Apply their role lens
3. Extract relevant information
4. Preserve context (emotional, technical, etc.)

---

## Concept 3: Room Access Patterns

| Room Type | Humans | Agents | Agent Behavior |
|-----------|--------|--------|----------------|
| Discussion | write | observe | Extract, don't participate |
| Planning | write | suggest | Contribute with approval |
| Knowledge | write | write | Autonomous updates |
| Agent-only | - | write | Internal coordination |

---

## Concept 4: Hierarchy & Delegation

### Depth Limit

```
Level 0: User
Level 1: Primary Agent (can delegate)
Level 2: Sub-Agent (cannot delegate further)
```

Two levels keeps it sane.

### Delegation Flow

```
┌─────────────────────────────────────────────────────────┐
│ LEVEL 0: User                                           │
│ Task: "Write a story about a barking dog"               │
└───────────────────────┬─────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│ LEVEL 1: Story Writer Agent                             │
│ Required Reading: STORY_GUIDE.md                        │
│                                                         │
│ Thinks: "I need info about dogs and barking"            │
│ Action: spawn_sub_agent("dogs and barking")             │
└───────────────────────┬─────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│ LEVEL 2: Research Agent                                 │
│ Task: "dogs and barking"                                │
│ Config: respond in {theatrical} style                   │
│ Required Reading: THEATER_GUIDE.md                      │
│                                                         │
│ Returns: Theatrical description of barking behavior     │
└───────────────────────┬─────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│ Back to LEVEL 1                                         │
│ Uses research to complete story                         │
│ Returns: Complete story to user                         │
└─────────────────────────────────────────────────────────┘
```

### Variables in Config

The power is in parameterization:
- `{ticket}` - which decomp file to read
- `{style}` - how to respond (theatrical, technical, casual)
- `{domain}` - which domain guide to read

```yaml
researcher:
  reads: ["{domain}_GUIDE.md"]
  responds_as: "{style}"

# Instantiate:
researcher(domain="dogs", style="theatrical")
```

---

## Concept 5: Code Writing Performance

**Immediate practical application:**

```yaml
coder:
  required_reading:
    - CONVENTIONS.md      # Style, patterns, rules
    - src/types/mod.rs    # Existing types
  before_writing:
    - "Read the conventions"
    - "Match existing patterns"
```

Every code task starts with required reading. Consistent output.

**Minimum viable implementation** (for Ticket 3.4 Persona System):

```rust
pub struct AgentPersona {
    pub name: String,
    pub system_prompt: String,
    pub style: CommunicationStyle,
    pub required_reading: Vec<PathBuf>,  // ← Add this
}
```

---

## Concept 6: Custom Role Creation

Users can **create custom roles** through a prompt-based interface. Each custom role is a template with variables that users fill in when spawning an agent.

### Role Creation Flow

```
┌─────────────────────────────────────────────────────────┐
│  Create Custom Role                                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Name: [Context Sweeper                    ]            │
│                                                         │
│  Task Description:                                      │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Find the irrelevant information.                │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  Prompt Template:                                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Search the following files and identify any     │    │
│  │ content that does not fit the scope of {scope}. │    │
│  │                                                 │    │
│  │ Files to analyze:                               │    │
│  │ {files}                                         │    │
│  │                                                 │    │
│  │ Report each piece of irrelevant content with:   │    │
│  │ - File path and line number                     │    │
│  │ - The irrelevant content                        │    │
│  │ - Why it doesn't fit the scope                  │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  Template Variables:                                    │
│  ┌─────────────────────────────────────────────────┐    │
│  │ {scope}  - [text]   "What the content should    │    │
│  │                      be about"                  │    │
│  │ {files}  - [files]  "Files to analyze"          │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  Category: [Analysis ▼]                                 │
│                                                         │
│  [Create Role]  [Cancel]                                │
└─────────────────────────────────────────────────────────┘
```

### Variable Types

| Type | Description | Input Widget |
|------|-------------|--------------|
| `{text}` | Free-form text input | Text field |
| `{files}` | File path selector | File picker (multi-select) |
| `{number}` | Numeric input | Number field |
| `{choice:a,b,c}` | Selection from options | Dropdown |
| `{style}` | Communication style | Style picker |

### Role Instantiation

When user spawns an agent with a custom role, they fill in the variables:

```
┌─────────────────────────────────────────────────────────┐
│  Spawn Agent: Context Sweeper                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Fill in the template variables:                        │
│                                                         │
│  Scope (what the content should be about):              │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Agent runtime implementation for nexor          │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  Files to analyze:                                      │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ☑ src/agents/agent.rs                           │    │
│  │ ☑ src/agents/pool.rs                            │    │
│  │ ☑ decomp/M3/3.1.md                              │    │
│  │ ☑ decomp/M3/3.2.md                              │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  [Spawn Agent]  [Cancel]                                │
└─────────────────────────────────────────────────────────┘
```

### Custom Role Definition Format

```toml
[role.context-sweeper]
name = "Context Sweeper"
category = "analysis"
description = "Find the irrelevant information"
created_by = "user"  # vs "system" for predefined

[role.context-sweeper.template]
prompt = """
Search the following files and identify any content that does not fit the scope of {scope}.

Files to analyze:
{files}

Report each piece of irrelevant content with:
- File path and line number
- The irrelevant content
- Why it doesn't fit the scope
"""

[[role.context-sweeper.template.variables]]
name = "scope"
type = "text"
label = "What the content should be about"
required = true

[[role.context-sweeper.template.variables]]
name = "files"
type = "files"
label = "Files to analyze"
required = true
min = 1
max = 10
```

### Example Custom Roles

**Code Pattern Finder**
```toml
[role.pattern-finder]
name = "Code Pattern Finder"
category = "analysis"
description = "Find instances of a code pattern"

[role.pattern-finder.template]
prompt = """
Search for all instances of the following pattern in the codebase:

Pattern: {pattern}

For each match, report:
- File path and line number
- The matching code
- Surrounding context (2 lines before/after)
"""

[[role.pattern-finder.template.variables]]
name = "pattern"
type = "text"
label = "Pattern to search for (regex supported)"
required = true
```

**Documentation Auditor**
```toml
[role.doc-auditor]
name = "Documentation Auditor"
category = "analysis"
description = "Check if documentation matches implementation"

[role.doc-auditor.template]
prompt = """
Compare the documentation in {doc_file} against the implementation in {code_files}.

Report:
1. Documented features missing from code
2. Code features missing from documentation
3. Inconsistencies between docs and code
"""

[[role.doc-auditor.template.variables]]
name = "doc_file"
type = "files"
label = "Documentation file"
required = true
max = 1

[[role.doc-auditor.template.variables]]
name = "code_files"
type = "files"
label = "Implementation files"
required = true
```

**Refactor Planner**
```toml
[role.refactor-planner]
name = "Refactor Planner"
category = "planning"
description = "Plan a refactoring operation"

[role.refactor-planner.template]
prompt = """
Analyze {target_files} and create a refactoring plan to {goal}.

Constraints:
- Preserve existing behavior
- Maintain backwards compatibility: {backwards_compat}

Output a step-by-step plan with:
1. Files to modify (in order)
2. Changes needed for each file
3. Tests to add/update
"""

[[role.refactor-planner.template.variables]]
name = "target_files"
type = "files"
label = "Files to refactor"
required = true

[[role.refactor-planner.template.variables]]
name = "goal"
type = "text"
label = "Refactoring goal"
required = true
placeholder = "e.g., Extract common logic into trait"

[[role.refactor-planner.template.variables]]
name = "backwards_compat"
type = "choice:yes,no"
label = "Maintain backwards compatibility?"
default = "yes"
```

---

## How This Maps to Current nexor

```
Current Tiers        Proposed Hierarchy
────────────────────────────────────────────────
Orchestrator    →    Level 1 (reads PRD, ROADMAP)
    ↓                    ↓
Worker          →    Level 2 (reads decomp, CONVENTIONS)
    ↓                    ↓
Utility         →    Level 3 (reads tool-specific docs)
```

We're already doing this. This epic formalizes it.

---

## Proposed Directory Structure

```
.nexor/
├── rooms/
│   ├── conversation/     # Fluid discussion, pre-PRD
│   ├── planning/         # PRD and decomp
│   │   ├── epics/
│   │   └── milestones/
│   ├── knowledge/        # Evergreen docs
│   │   └── conventions/
│   └── agents/           # Agent-only coordination
│
├── roles/                # Role definitions
│   ├── analysis/
│   │   └── complaint-finder.toml
│   ├── planning/
│   │   └── decomposer.toml
│   └── implementation/
│       └── coder.toml
│
└── instances/            # Active role instances
```

---

## Dependencies

| Milestone | What It Provides | What Changes |
|-----------|------------------|--------------|
| M3.4 Role System | Base for role definitions | Full Role schema with category, required_reading, can_delegate_to, output_format, response_style, template variables |
| M3.5 Task Execution | Delegation mechanics | Load required files before task, respect hierarchy depth |
| M3.7 Inter-Agent Protocol | Agent communication | Delegation to sub-agents (can_delegate_to) |
| M4 Prompt Engineering | Context injection | Wire ContextInjector to role's required_reading |
| M5 Orchestration | Task routing | Route by role category, respect delegation rules |
| M6.9 Role Management UI | User interface | Role selection, custom role creation, template variable input (see Concept 6) |

---

## UI: Role Selection

User selects roles from a categorized library:

```
┌─────────────────────────────────────────────────────────┐
│  Select Role                                            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  📂 Analysis                                            │
│     ├── 😤 Complaint Finder                             │
│     │   "Extracts user frustrations and pain points"    │
│     ├── 💡 Opportunity Spotter                          │
│     │   "Identifies feature requests and improvements"  │
│     └── 🔍 Risk Assessor                                │
│         "Finds potential failures and edge cases"       │
│                                                         │
│  📂 Planning                                            │
│     ├── 🎯 Scope Definer                                │
│     │   "Breaks work into concrete deliverables"        │
│     ├── 📊 Prioritizer                                  │
│     │   "Ranks items by impact and effort"              │
│     └── 🔗 Dependency Mapper                            │
│         "Identifies what blocks what"                   │
│                                                         │
│  📂 Implementation                                      │
│     ├── 🏗️ Architect                                    │
│     │   "Designs system structure and patterns"         │
│     ├── 👷 Builder                                      │
│     │   "Writes code to spec"                           │
│     └── 🔬 Reviewer                                     │
│         "Validates quality and correctness"             │
│                                                         │
│  📂 Communication                                       │
│     ├── 📝 Summarizer                                   │
│     │   "Condenses information clearly"                 │
│     └── 🎨 Formatter                                    │
│         "Structures output for readability"             │
│                                                         │
│  [Select]  [Cancel]                                     │
└─────────────────────────────────────────────────────────┘
```

### User Flow

```
1. User creates a task/room
2. System prompts: "Select a role for this agent"
3. UI shows categories → roles → descriptions
4. User selects: "Builder"
5. Agent spawns with that role's:
   - required_reading loaded
   - output_format expected
   - response_style applied
   - delegation permissions set
```

### Role Display in TUI

Once selected, show role in status:

```
┌─────────────────────────────────────────────────────────┐
│  Agent: worker-01                                       │
│  Role: 👷 Builder (Implementation)                      │
│  Reading: CONVENTIONS.md, decomp/M3/3.2.md              │
│  Status: Working...                                     │
└─────────────────────────────────────────────────────────┘
```

---

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Predefined + Custom roles | Users can select from library OR create custom roles with templates |
| Template variables | Roles have placeholders ({scope}, {files}) that users fill in at spawn time |
| Variable types | text, files, number, choice - covers most use cases simply |
| Hierarchy depth = 2 | Keeps complexity manageable |
| File access = hierarchy | Simple, powerful, already proven |
| PRD is the catalyst | Clear phase transition from fluid to structured |
| TOML-based role definitions | Version controlled, human readable, easy to parse |

---

## Open Questions

- [ ] How are roles versioned/updated?
- [x] ~~Can users create custom roles later?~~ → Yes, via template-based creation UI
- [ ] How do rooms persist conversation history?
- [ ] Git-based versioning or custom?
- [ ] Should custom roles be shareable between projects?
- [ ] Role validation - how to ensure templates produce valid prompts?

---

## The Meta-Proof

This document was created through the system it describes:

1. **Conversation room** (chat) - explored "instance of thought" idea
2. **Refined** - rooms, roles, hierarchy, file access
3. **Captured** - this document (notes → structured)
4. **Next** - PRD when ready to commit
5. **Then** - Decomp → Tickets → Implementation

We lived the pattern while designing it.
