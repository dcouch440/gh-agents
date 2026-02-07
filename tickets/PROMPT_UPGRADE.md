# Prompt Upgrade Plan

Upgrade all framework-level prompts to enterprise grade based on research findings.

**References:**
- `docs/PROMPT_CATALOG.md` — Full catalog of all 20 prompts with file paths and current text
- `docs/PROMPT_RESEARCH.md` — Research findings, techniques, and quantitative results

---

## Phase 1: Protocol Prompts

**Files:** `src/server/hub/protocols/prompt_gen.rs`
**Impact:** Critical — these drive every workflow execution

### 1A: Decomposition Protocol (`decomp_prompt()`, lines 7-59)

**Current problems:**
- No guidance on when to split vs. keep together (causes over/under-decomposition)
- No cooperative language ("another agent will handle the rest")
- No instruction to consider task dependencies between subtasks
- Agent descriptions use inline markdown bold — not the structured format that doubled selection accuracy in AutoGen research
- No think-before-acting pattern

**Changes based on research:**
- Add calibration principle: "Decompose into the minimum subtasks needed — each should be a self-contained unit of work"
- Add dependency awareness: "Consider whether subtasks have ordering constraints"
- Add cooperative framing: "Each agent will execute independently with only the context you provide"
- Restructure agent descriptions with structured format (name, capability summary, tools)
- Add think-before-decomposing guidance using `<thinking>` pattern
- Use moderately specific verbs: "analyze," "break down," "identify" (not "microscopically examine")

### 1B: Routing Protocol (`route_prompt()`, lines 63-92)

**Current problems:**
- No "think before routing" step — just "examine and route"
- No decision criteria or confidence signal
- Research shows routing to unbounded categories drops to 53% accuracy
- Agent descriptions lack the structured format

**Changes based on research:**
- Add think-before-routing: "First, identify the core intent and requirements of the input"
- Add decision criteria framing: describe what makes input a match for each agent
- Restructure agent descriptions to capability-focused format
- Add disambiguation guidance for edge cases

### 1C: Review Protocol (`review_prompt()`, lines 96-118)

**Current problems:**
- No rubric or criteria definitions — just "review and decide"
- No severity scale — recipe for rubber-stamping or nitpicking
- No explanation-first pattern (research shows this reduces variance)
- No guidance on what constitutes quality

**Changes based on research:**
- Add integer rubric with score definitions per decision option
- Add explanation-first pattern: reason tied to criteria, then output decision
- Add single-criterion focus guidance
- Add specific evaluation dimensions (correctness, completeness, clarity)
- Add anti-rubber-stamping language: "Form your own independent assessment"

### 1D: Transform Protocol (`transform_prompt()`, lines 122-136)

**Current problems:**
- Only 3 lines — minimal guidance
- No examples, no format guidance, no chain-of-thought

**Changes based on research:**
- Add processing steps: "Analyze the input, identify the relevant data, map it to the schema"
- Add positive output framing with WHY context
- Reference the schema description meaningfully

---

## Phase 2: Review & Verification Prompts

**Files:** `src/server/hub/engine/filters/debate_verification/mod.rs`
**Impact:** High — these are the quality gates

### 2A: Verifier System Prompt (lines 96-119)

**Current problems:**
- JSON example shows `"approved": true` while asking for critique — contradictory signal
- No severity scale definitions (what IS "high" vs "medium" vs "low"?)
- Asks about everything at once — research says single-criterion focus is better
- Mixes markdown headers with JSON awkwardly

**Changes based on research:**
- Define severity levels explicitly (high = blocks correctness, medium = reduces quality, low = style/minor)
- Use explanation-first pattern: reason about the output, then produce judgment
- Show the JSON example with `"approved": false` to model the critical review behavior
- Wrap in XML tags consistent with rest of system

### 2B: Verifier User Message (lines 122-133)

**Current problems:**
- Uses markdown headers in what lands in user message context
- "Review this response from your area of expertise" is vague

**Changes based on research:**
- Wrap sections in XML tags (`<original_task>`, `<response_under_review>`)
- Add specific evaluation instruction: "Evaluate for factual accuracy, logical consistency, and completeness"

### 2C: Merged Feedback Format (lines 136-163)

**Current problems:**
- Uses markdown headers — should use XML for consistency
- No prioritization guidance for the primary agent

**Changes based on research:**
- Wrap in `<verification_feedback>` tags
- Add prioritization: "Address HIGH severity issues first"
- Add instruction to preserve what's working: "Retain approved aspects of your original response"

---

## Phase 3: Schema & Output Enforcement

**Files:** `src/server/hub/dag/mod.rs`, `src/server/hub/engine/filters/schema_enhancement/mod.rs`
**Impact:** Medium-High — affects every step with structured output

### 3A: Schema Enforcement Prompt (dag/mod.rs lines 1671-1674, 3197)

**Current problems:**
- "Respond ONLY with the JSON object, no other text" — negative framing
- No reasoning field guidance (research shows 33% → 92% improvement)
- Overlaps with schema_enhancement filter

**Changes based on research:**
- Positive framing: "Your response will be parsed directly by a JSON parser" (WHY context)
- Add note about field ordering if schema includes reasoning field
- Deduplicate with schema_enhancement filter — ensure they complement rather than repeat

### 3B: Schema Enhancement Filter (schema_enhancement/mod.rs lines 40-49)

**Current problems:**
- 5 out of 6 rules are "Do NOT" — research shows negative instructions are less effective
- No WHY context for any rule

**Changes based on research:**
- Reframe all rules positively with WHY context:
  - "Do NOT wrap in markdown" → "Output raw JSON. The consumer parses your response directly with a JSON parser, so any surrounding text causes parsing errors."
  - "Do NOT include explanatory text" → "The entire response body is passed to JSON.parse(). Only valid JSON will succeed."
- Consolidate from 6 rules to 3-4 high-signal positive instructions

### 3C: Schema Validation Retry Filter (schema_validation_retry/mod.rs lines 45-75)

**Current problems:**
- Retry messages are adequate but could include WHY context
- No positive reframing

**Changes based on research:**
- Add WHY to each error: "The downstream system received your response and failed to parse it because..."
- Keep error-specific messages but add positive guidance on what success looks like

---

## Phase 4: Mode Resolver & Utility Prompts

**Files:** `src/server/hub/mode_resolver/mod.rs`, `src/server/tools/mod.rs`
**Impact:** Medium — affects routing accuracy and auxiliary operations

### 4A: Mode Classification Prompt (mode_resolver/mod.rs lines 265-285)

**Current problems:**
- No thinking step before classification
- Ambiguous output format — "output ONLY the mode key" (JSON? raw string?)
- No example of what each mode looks like in practice

**Changes based on research:**
- Add brief analysis step: "Identify the primary intent of the input"
- Clarify output format explicitly
- Consider adding one-line usage examples per mode

### 4B: Haiku Summarization (tools/mod.rs lines 520-542)

**Current problems:**
- "Summarize in 2-3 sentences. Be concise." — no role, no purpose context

**Changes based on research:**
- Add purpose: "This summary will be used for search indexing, so include key entities and topics"
- Add role for consistency

### 4C: Haiku Context Extraction (tools/mod.rs lines 575-597)

**Current problems:**
- Returns magic string "No prior context needed" that's matched with string literal in chat strategy
- Could benefit from clearer output specification

**Changes based on research:**
- Structured output format or clearer sentinel value
- Add purpose context: "The extracted context will be prepended to a new conversation turn"

### 4D: Haiku Title Generation (tools/mod.rs lines 545-571)

**Current problems:**
- Minimal but functional

**Changes based on research:**
- Add purpose context for better titles
- Minor — low priority

---

## Phase 5: Filter Consistency Pass

**Files:** All filter `mod.rs` files
**Impact:** Low — these already work well

### 5A: Reasoning Trace Filter

- Already good. Verify field ordering guidance aligns with schema research.

### 5B: Few-Shot Filter

- Already good. Verify `<examples>` tag usage aligns with Anthropic recommendations.

### 5C: Agent Guidance Filter

- Already good. No changes needed.

---

## Verification

After each phase:
```bash
~/.cargo/bin/cargo check
~/.cargo/bin/cargo test          # Full suite
~/.cargo/bin/cargo clippy
~/.cargo/bin/cargo fmt
```

Phase-specific test targets:
- Phase 1: `cargo test hub::protocols::`
- Phase 2: `cargo test hub::engine::filters::debate_verification::`
- Phase 3: `cargo test hub::engine::filters::schema_enhancement::` + `cargo test hub::engine::filters::schema_validation_retry::`
- Phase 4: `cargo test hub::mode_resolver::` + `cargo test server::tools::`
- Phase 5: `cargo test hub::engine::filters::`
