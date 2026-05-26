# Workflow Agent Tools — From Shell-Only to Assisted Shell

## The Problem

The workflow agent has one tool for all file operations: `run_command`. It writes `topology.json` and `nodes/*.md` via shell heredocs passed through `sh -c`. This works most of the time, but fails in a specific, recurring way.

The failure is structural, not behavioral. The LLM must produce valid POSIX shell syntax inside a JSON string value. A heredoc command like:

```
cat > nodes/research.md << 'EOF'
Research pricing for the top 5 PM tools.
EOF
```

must be encoded as a JSON string where `\n` represents actual newlines. But LLMs routinely produce `\\n` (literal backslash-n) instead, or wrap the entire command in extra double quotes. The shell then sees a single unbroken line instead of a multi-line heredoc, and the command fails with exit code 127.

When the first write in a `&&` chain fails, topology.json exists but node files are empty. The post-command validation reports "file does not exist" for every missing node on every subsequent tool call. The agent sees these errors and retries the same broken command. This burns 3-5 rounds before the engine's failure hint fires — by which point, the agent has used a third of its round budget on a problem it can't fix by trying harder.

The user then intervenes ("go ahead and finish"), and the agent spends several more rounds re-reading every file via `cat` to understand what was already written before it can resume.

The agent's intent is always correct. The node text it generates is good. The topology structures are well-designed. The failure is purely mechanical — the encoding boundary between JSON and shell.

## The Insight

The workflow agent's workspace is two things: one JSON file and a handful of one-sentence markdown files. It doesn't need a shell to write these. It needs a shell for everything else.

Claude Code has this right. It gives agents dedicated tools for common operations (Read, Write, Edit) and a shell (Bash) for everything else. The dedicated tools handle the structured operations reliably. The shell handles the long tail — searching, inspecting, running scripts. Neither replaces the other.

The workflow agent should follow the same pattern: file tools for the common case, shell for the uncommon case.

## Design

### Tools

**`read_file`** — Read a single file by path. The agent should read before writing, just like in any collaborative editing system. It sees what the user changed on the canvas before making its own edits.

```
read_file({ "path": "topology.json" })
→ "{\n  \"nodes\": {\n    \"research\": { \"depends_on\": [] }\n  }\n}"

read_file({ "path": "nodes/research.md" })
→ "Research pricing for the top 5 PM tools."
```

**`write_file`** — Write a single file by path. Content is a JSON string value — newlines are native, no heredocs, no shell encoding. The tool creates parent directories if needed.

```
write_file({ "path": "topology.json", "content": "{\n  \"nodes\": {\n    ... }\n}" })
write_file({ "path": "nodes/research.md", "content": "Research pricing for the top 5 PM tools." })
```

**`run_command`** — Shell access for everything else. Searching (`grep`), listing (`ls`), inspecting file structure, or any ad-hoc operation the agent needs. The agent should use `read_file` and `write_file` for file I/O instead of `cat` and heredocs, but the shell remains available when it's the right tool.

**`think`** — Unchanged. Internal reasoning with no side effects.

**`render_panel`** — Unchanged. Interactive panels for user decisions.

### Why Not Batch?

A `write_files` (plural) tool that writes multiple files in one call is tempting — the agent could create the entire board in a single tool call. But it fights the system's collaborative design:

- **Read before write.** The agent should read `topology.json` before rewriting it. With a batch tool, it's tempted to skip the read and blast a full board in one shot, ignoring what the user may have changed on the canvas.
- **Iterative editing.** In a multi-turn conversation, the agent edits one node, adds a dependency, tweaks the topology. These are individual file operations. A batch tool that requires listing every file encourages full rewrites instead of targeted edits.
- **Error localization.** When `write_file` fails, you know exactly which file failed. When `write_files` fails, the error could be any of the files — and the agent has to figure out which one.

The agent can still write multiple files in one turn by making multiple `write_file` calls. The engine supports multiple tool calls per round. But each write is deliberate and individually validated.

### Why Keep run_command?

The shell covers the long tail:

- **Searching.** `grep -r "pricing" nodes/` to find which nodes mention a topic.
- **Listing.** `ls nodes/` to see what exists.
- **Inspecting.** `wc -l nodes/*.md` to check file sizes.
- **Ad-hoc operations.** Anything we haven't anticipated.

Without the shell, the agent is constrained to exactly the operations we defined. With it, the agent can handle edge cases we didn't predict. The shell is the escape hatch.

The key change: the agent should prefer `read_file`/`write_file` for standard file I/O and fall back to `run_command` for everything else. The system prompt and tool descriptions make this preference clear.

### Per-Agent Tool Definitions

The current `run_command` is a shared definition used by every agent type — workflow agent, system node agent, runtime agents. Its description is a 50-line shell tutorial covering pip, npm, git, sqlite3, gcc, curl, archives, data processing, and more. The workflow agent sees all of this every turn.

This is a likely cause of behavioral drift. The tool description is effectively a competing system prompt. When you tell an agent "Available tools: python, pip, node, npm, git, curl, wget, jq, grep, sed, awk, find, xargs, sort, uniq, wc, head, tail, tee, tr, cut, zip, unzip, sqlite3, make, gcc" — you're setting expectations about capabilities that don't match the workflow agent's simple file I/O job. The first example in the description is a heredoc — actively teaching the exact pattern that breaks.

Each agent type should get its own `run_command` definition, scoped to its actual job. The workflow agent's `run_command` needs two lines: "Run a shell command for searching or inspecting the workspace. For file reads and writes, use read_file and write_file." No heredoc examples, no package managers, no 50-line tutorial.

### input_examples

Anthropic's tool use research shows that adding `input_examples` to tool definitions improves accuracy from 72% to 90% on complex parameters. The `Tool` struct should support an optional `input_examples` field:

```rust
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_examples: Option<Vec<serde_json::Value>>,
}
```

Examples teach the model the exact JSON format to produce. They belong on the tool itself — not in tool descriptions, not in the system prompt. The system prompt teaches *what to build* (workflow design). `input_examples` teaches *how to call the tool* (JSON format).

**`write_file` input_examples:**
```json
[
  {
    "path": "topology.json",
    "content": "{\n  \"nodes\": {\n    \"research\": { \"depends_on\": [] },\n    \"report\": { \"depends_on\": [\"research\"] }\n  }\n}"
  },
  {
    "path": "nodes/research.md",
    "content": "Research pricing for the top 5 PM tools."
  }
]
```

**`read_file` input_examples:**
```json
[
  { "path": "topology.json" },
  { "path": "nodes/research.md" }
]
```

**`run_command` input_examples (workflow agent version):**
```json
[
  { "command": "ls nodes/" },
  { "command": "grep -rn 'pricing' nodes/" }
]
```

No heredocs in any example. The model never sees a heredoc pattern — it has no reason to produce one.

**`think` input_examples:**
```json
[
  { "thought": "The user wants pricing and features compared. These are independent — they can run in parallel. I need a fan-out from the research, then a merge for the comparison." }
]
```

**`render_panel` input_examples:**
```json
[
  {
    "content": "# Competitive Analysis\n\n## What to research\n- [ ] Pricing\n- [ ] Features\n\n## Scope\n- [> Competitors (e.g. \"top 5 PM tools\")]",
    "submit_label": "Build workflow"
  }
]
```

### Two Layers of Examples

`input_examples` on the tool and scenario examples in the system prompt serve different purposes:

| Layer | Purpose | Teaches |
|-------|---------|---------|
| `input_examples` on tool | JSON format — what the tool call looks like | How to call the tool |
| System prompt `<examples>` | Workflow design — when to fan out, when to verify | What to build and why |

The system prompt examples still show tool calls (using `write_file` instead of `run_command` heredocs), but their value is the reasoning around them: "The story: research each dimension, verify, then report. Pricing and features are independent — parallel." The `input_examples` ensure the model gets the JSON right. The system prompt ensures it gets the workflow right.

### Validation

The existing validation pipeline stays. After each `write_file`:
- Snapshot before/after for change detection
- Validate topology JSON structure
- Validate node content (non-empty)
- Cross-reference checks (topology ↔ node files)
- Sync to DB if files changed

One adjustment: suppress cross-reference "file does not exist" errors when they're clearly caused by a partial write in progress (topology written, node files coming next). Only surface cross-reference errors when the agent's turn is complete — in `on_complete`, not after every tool call. This eliminates the validation avalanche that causes retry loops.

### Workspace Setup

`project_to_repo()` already creates `topology.json` (with `{"nodes": {}}`) and the `nodes/` directory on first turn. The system prompt should tell the agent this: "Your workspace starts with `topology.json` and `nodes/` already in place."

### System Prompt Changes

The `<role>` section updates to describe the new tool set:

```
You help users design workflows through conversation. You work in a
repository that syncs live to the user's visual canvas — when you
write files, nodes and edges appear on their screen in real-time.

Your workspace:
  topology.json        — node dependency graph
  nodes/{slug}.md      — one text file per node

Use read_file to see a file. Use write_file to create or update a
file. Use run_command for search, listing, or other shell tasks.
The workspace starts ready — topology.json and nodes/ already exist.
Read before writing — the user may have edited the canvas.
```

The `<examples>` section rewrites tool calls from heredocs to `write_file`:

```xml
<tool_call name="write_file">
{"path": "topology.json", "content": "{\n  \"nodes\": {\n    \"research_pricing\": { \"depends_on\": [] },\n    \"write_report\": { \"depends_on\": [\"research_pricing\"] }\n  }\n}"}
</tool_call>
<tool_call name="write_file">
{"path": "nodes/research_pricing.md", "content": "Research pricing for the top 5 PM tools."}
</tool_call>
<tool_call name="write_file">
{"path": "nodes/write_report.md", "content": "Write the competitive analysis with recommendations."}
</tool_call>
```

The `render_panel` examples stay unchanged.

## What Changes

| Aspect | Current | New |
|--------|---------|-----|
| File writes | `run_command` + heredocs | `write_file` (JSON string content) |
| File reads | `run_command` + `cat` | `read_file` (structured) |
| Search/listing | `run_command` | `run_command` (unchanged) |
| Encoding failures | Common (JSON ↔ shell boundary) | Eliminated for file I/O |
| Partial write noise | Validation avalanche every tool call | Cross-ref deferred to turn completion |
| Tool examples | In system prompt as heredoc blocks | On tool definitions via `input_examples` |
| Tool count | 3 (run_command, think, render_panel) | 5 (read_file, write_file, run_command, think, render_panel) |

## What Doesn't Change

- The collaborative model. Agent reads, edits, writes. User edits the canvas. Both see the same board.
- The three-layer system. Workflow agent → system node agent → runtime agents.
- The snapshot/sync mechanism. File changes detected via before/after diffs, synced to DB immediately.
- The validation pipeline. Topology structure, node content, cross-references, cycle detection.
- The `<current_state>` rebuild. Fresh XML every turn from DB state.
- The philosophy, nodes, topology, patterns, and guide sections of the system prompt.

## Implementation

1. Add `input_examples: Option<Vec<Value>>` to `Tool` struct, update provider adapters
2. Add `read_file_tool()` and `write_file_tool()` to tool registry with `input_examples`
3. Add `handle_read_file()` and `handle_write_file()` to `WorkflowAgentStrategy`
4. Defer cross-reference validation to `on_complete` (remove from per-command validation)
5. Update `tools()` to include all five tools
6. Update `config/workflow_agent/system.md` — role section and examples
7. Tests for new handlers
