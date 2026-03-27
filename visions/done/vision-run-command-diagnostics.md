# Run Command Diagnostics — Vision

## What It Is

A diagnostics library that wraps every agent shell command with rich, structured feedback. Instead of raw stdout/stderr, agents receive filesystem diffs, validation results, no-op warnings, loop detection, and workspace awareness — everything an LLM needs to make good decisions without burning round trips on exploration.

The library sits between the container execution layer and the agent. The agent sends a command. The diagnostics engine snapshots the workspace, executes, diffs the result, runs analyzers, and returns a structured envelope. The agent sees what happened, not just what the command printed.

## Why This Matters

**No existing AI agent does this.** SWE-Agent has edit-time linting and output templates. Devin captures file diffs in replays but doesn't feed them back into the agent's immediate observation. OpenHands has typed observations but no enrichment. Claude Code's hooks skip on failed commands — exactly where diagnostics matter most. Raw stdout/stderr is optimized for humans at terminals, not LLMs in a loop.

**Silent failures are the #1 threat.** `sed -i 's/foo/bar/' file.json` returns exit code 0 even when the pattern doesn't match. `grep` returns empty output. `cp` to a wrong path succeeds. The agent gets positive reinforcement and proceeds on false assumptions. Every downstream decision is built on a lie.

**Agents waste round trips on exploration.** After running a command, an agent typically runs `ls` to see what changed, `cat` to verify edits, `jq -e` to validate JSON, `diff` to compare files. That's 4 round trips (4 LLM calls) that the system could eliminate by observing and reporting automatically.

**Escalation loops are the #2 threat.** Research shows agents that don't solve by turn 10 almost always spiral (Pragmatic Engineer analysis). AgentRefine (arXiv:2501.01702) found agents "frequently stuck in the same mistake for a long while." A broken edit leads to a worse fix which leads to a worse fix. Without a circuit breaker, the agent burns tokens until timeout.

**Container model creates invisible traps.** Each `docker exec` is a fresh shell. Environment variables, working directories, aliases — all vanish between commands. Agents repeatedly stumble on this because nothing tells them. Pre-execution detection costs almost nothing and prevents the most common failures.

## The Diagnostics Pipeline

Every command flows through three phases:

```
┌─────────────────────────────────────────────────────┐
│                 DiagnosticsEngine                    │
│                                                      │
│  1. PRE-EXECUTION                                    │
│     Parse command → pattern match → warn/block        │
│     "cd won't persist" / "this will hang"            │
│                                                      │
│  2. EXECUTION                                        │
│     Snapshot upper dir → run command → diff upper dir │
│     Capture stdout, stderr, exit code, wall time     │
│                                                      │
│  3. POST-EXECUTION                                   │
│     No-op detection → validation → classification    │
│     → truncation → loop check → workspace digest     │
│                                                      │
│  → Assemble CommandEnvelope                          │
└─────────────────────────────────────────────────────┘
```

### Pre-Execution: Catch Problems Before They Happen

Parse the command string and match against known anti-patterns. These are problems inherent to the container execution model that agents will hit repeatedly.

**State persistence warnings:**
```
Agent sends: export DATABASE_URL=postgres://localhost/mydb

Pre-check detects: `export` in a non-persistent shell
Warns: "Environment variables don't persist between commands.
        Use 'DATABASE_URL=value python script.py' on one line,
        or write to a .env file and source it per command."
```

```
Agent sends: cd /workspace/src/app

Pre-check detects: `cd` as a standalone command (not chained with &&)
Warns: "cd doesn't persist between commands. Use
        'cd /workspace/src/app && python main.py' on one line,
        or use absolute paths."
```

Anthropic's own SWE-bench analysis found that requiring absolute paths was critical for agent success. This single check prevents the most common agent confusion.

**Interactive command detection:**
```
Agent sends: python

Pre-check detects: `python` with no script argument
Warns: "This opens an interactive REPL that will hang until timeout.
        Use 'python script.py' or 'python -c \"code\"'"
```

```
Agent sends: mysql -u root

Pre-check detects: known interactive CLI without redirect/command flag
Warns: "mysql without -e flag opens interactive mode.
        Use 'mysql -u root -e \"SELECT ...\"'"
```

Known interactive commands: `python` (no args), `node` (no args), `mysql` (no -e), `psql` (no -c), `ssh`, `ftp`, `telnet`, `irb`, `ghci`. Also detect `apt-get install` without `-y` and auto-prepend `DEBIAN_FRONTEND=noninteractive`.

**Shell compatibility detection:**
```
Agent sends: if [[ -f config.json ]]; then echo "exists"; fi

Pre-check detects: `[[` (bash-specific) in an `sh` environment
Warns: "This environment uses 'sh', not 'bash'.
        Use '[ -f config.json ]' instead of '[[ ]]'"
```

Also catches: `source` (use `.`), bash arrays, `<()` process substitution, `nvm`/`rvm`/`conda activate` (shell functions not available in `sh -c`).

### Execution: Observe Everything

The execution phase is thin — snapshot, run, diff.

**1. Checkpoint the upper directory:**
```
cp -a /tmp/overlay/upper /tmp/overlay/upper-checkpoint-N
```

For 100 files: ~50ms. For 1000 files: ~200ms. Negligible compared to LLM latency. This gives us a rollback point if the command corrupts the workspace.

**2. Execute the command.** Capture stdout, stderr, exit code, wall time separately.

**3. Walk the OverlayFS upper directory.** The upper dir IS the diff — no filesystem comparison needed. This is the same technique BuildKit switched to (moby/buildkit#2181) because it's dramatically faster than full filesystem walks.

- New files in upper that aren't in base → Created
- Files in upper that exist in base → Modified (copy-up happened)
- Whiteout files (char device 0/0 or `.wh.` prefix) → Deleted
- Opaque directory markers (`.wh..wh..opq`) → Directory replaced

Performance: O(changed files), not O(all files). Zero overhead during command execution — observation happens entirely after the command completes.

### Post-Execution: Analyze and Enrich

Six analyzers run against the command result and filesystem diff. Each is independent — they don't depend on each other.

---

## Feature 1: No-Op Detection

The single most impactful diagnostic. Detects when a command succeeds but does nothing useful.

### The Problem

```bash
$ sed -i 's/old_function/new_function/g' main.py
# exit 0, empty stdout, empty stderr
# Agent thinks: "edit worked, moving on"
# Reality: pattern didn't match, file unchanged
```

The agent proceeds with a false assumption. Every downstream decision is wrong. It might run tests expecting the fix to work, see failures, and start debugging the wrong thing.

### Detection Logic

Cross-reference three signals:

```
1. Command type (sed, grep, find, awk, cp, chmod)
2. Exit code (0 = "success")
3. Filesystem changes (none)
```

If the command type implies mutation + exit 0 + no filesystem changes → no-op.

**Mutation commands** (expect filesystem changes):
- `sed -i` — should modify a file
- `cp` / `mv` — should create/move files
- `chmod` / `chown` — should change metadata
- `mkdir` — should create a directory
- `tee` / redirect (`>`, `>>`) — should write a file

**Search commands** (expect stdout output):
- `grep` — should print matches
- `find` — should print paths
- `awk` / `sed` (without -i) — should print transformed text
- `cat` / `head` / `tail` — should print file content

**Package commands** (detect already-installed):
- `pip install` + "already satisfied" → no-op
- `npm install` + "up to date" → no-op
- `apt-get install` + "0 newly installed" → no-op

### Nearest Match Suggestions

When a pattern-based command (sed, grep) is a no-op, search the target file for similar patterns:

```
$ sed -i 's/scraper application/data pipeline/' agents/executor.json

result: success (no-op)
File agents/executor.json is unchanged.
Pattern "scraper application" not found.

Nearest matches in agents/executor.json:
  line 4:  "the scraper app built in step 1"
  line 9:  "scraper-application"
  line 15: "Scraper Application"
```

Implementation: extract the pattern from the command, run a fuzzy search (Levenshtein distance or substring matching) against the target file. Return the top 3 nearest matches with line numbers.

### Examples

**sed with no match:**
```
$ sed -i 's/old_func/new_func/g' main.py

result: success (no-op)
workspace changes: none
  sed made 0 replacements in main.py.
  Pattern "old_func" not found.
  Nearest: line 12 "oldFunction", line 45 "old_function"
```

**grep with no output:**
```
$ grep -r "import flask" src/

result: success (no-op)
0 matches for "import flask" in src/
  Nearest: 3 matches for "from flask" in src/
    src/app.py:1    from flask import Flask
    src/auth.py:2   from flask import request
    src/api.py:1    from flask import Blueprint
```

**cp to wrong path:**
```
$ cp config.json /workspace/app/config/

result: success
workspace changes:
  created: /workspace/app/config (file, not directory!)
  Note: /workspace/app/config/ (with trailing slash) doesn't exist as
  a directory. cp created a FILE named "config" — the source was
  renamed, not placed in a directory.
```

**pip already installed:**
```
$ pip install requests

result: success (no-op)
requests 2.31.0 was already installed. No new packages added.
```

---

## Feature 2: Pre-Execution Command Analysis

Pattern-match the command string before execution to catch known problems.

### The Problem

Agents waste entire round trips on commands that will definitely fail or mislead due to the container execution model. Each failed round trip costs an LLM call (~$0.01-0.10) and adds latency.

### Detection Categories

**Category 1: State Persistence**

Commands that set state in one shell but expect it in the next. Each `docker exec` is `sh -c "command"` — a fresh shell.

| Pattern | Detection | Warning |
|---------|-----------|---------|
| `export VAR=val` (standalone) | Regex: `^\s*export\s+\w+=` not followed by `&&` | Env vars don't persist. Use `VAR=val command` or `.env` file |
| `cd /path` (standalone) | Regex: `^\s*cd\s+` not followed by `&&` or `;` | cd doesn't persist. Chain with `&&` or use absolute paths |
| `alias name=cmd` | Regex: `^\s*alias\s+` | Aliases don't persist between commands |
| `source file` / `. file` | Regex: `^\s*(source\|\\.)\s+` | Source only affects this command's shell |

"Standalone" means the command is the entire input, not chained with `&&`. `cd /workspace/src && python main.py` is fine — the cd and python run in the same shell. `cd /workspace/src` alone is a no-op.

**Category 2: Interactive Commands**

Commands that wait for stdin and will hang until timeout.

| Pattern | Detection | Warning |
|---------|-----------|---------|
| `python` (no args) | Exact match or only flags (-v, -V allowed) | Opens interactive REPL. Use `python script.py` or `python -c` |
| `node` (no args) | Same pattern | Opens REPL. Use `node script.js` or `node -e` |
| `mysql` without `-e` | Regex: `mysql` without `-e` flag | Opens interactive client. Add `-e "SQL"` |
| `psql` without `-c` | Same pattern | Add `-c "SQL"` |
| `ssh`, `ftp`, `telnet` | Command prefix match | Interactive remote session, unsupported |
| `read -p` | Regex: `read\s+(-\w+\s+)*-p` | Waits for user input, will hang |
| `apt-get install` without `-y` | Missing `-y` flag | Will prompt for confirmation. Adding `-y` |

For `apt-get install` without `-y`, the system can auto-fix: prepend `DEBIAN_FRONTEND=noninteractive` and append `-y`. Warn the agent but don't block.

**Category 3: Shell Compatibility**

Bash-specific syntax that fails in `sh`.

| Pattern | Detection | Warning |
|---------|-----------|---------|
| `[[ ... ]]` | Regex: `\[\[` | Bash-specific. Use `[ ... ]` |
| `source file` | Regex: `\bsource\b` | Bash built-in. Use `. file` |
| `array=(...)` | Regex: `\w+=\(` | Bash arrays. Use space-separated strings |
| `<(...)` / `>(...)` | Regex: `[<>]\(` | Process substitution. Use temp files |
| `nvm use` / `rvm use` | Command prefix | Shell function, not available in sh -c |
| `conda activate` | Command prefix | Shell function, not available in sh -c |

### Pre-Check Behavior

Pre-checks produce warnings, not blocks. The command still executes (the agent might know what it's doing). The warning appears in the envelope before the result:

```
$ export API_KEY=secret123

pre-execution warning:
  Environment variables set with 'export' don't persist between
  commands in this environment. Use 'API_KEY=secret123 python script.py'
  on a single line, or write to a .env file.

result: success (no-op)
stdout: (empty)
workspace changes: none
```

Exception: known-hanging commands (bare `python`, `ssh`) could optionally block with a suggestion, saving the agent a 120-second timeout. This is configurable — the engine has a `block_interactive` flag.

---

## Feature 3: Escalation Loop Breaker

Detects when an agent is stuck editing the same file repeatedly and breaks the cycle.

### The Problem

Research evidence:
- Pragmatic Engineer: "the fewer turns an agent takes to solve an issue, the more likely it succeeds"
- AgentRefine (arXiv:2501.01702): agents "frequently stuck in the same mistake for a long while"
- Devin technical report: multi-file editing gaps where fixes for some files were correct but others were missed, creating cascading failures

The pattern:
```
cmd 1: Edit config.json → valid
cmd 3: Edit config.json → valid (different section)
cmd 5: Edit config.json → INVALID (broke JSON syntax)
cmd 7: Fix config.json  → still invalid (fixed wrong line)
cmd 9: Fix config.json  → worse (removed valid content)
```

Without intervention, this continues until the file is unrecognizable.

### Detection Logic

Track per-file edit history across commands within a step:

```rust
struct EditRecord {
    command_index: usize,
    valid_after: bool,
    checksum: String,
}

struct LoopDetector {
    file_edits: HashMap<PathBuf, Vec<EditRecord>>,
    error_history: Vec<String>,  // unique error messages across builds
}
```

**Thresholds:**

| Condition | Threshold | Action |
|-----------|-----------|--------|
| Same file edited N times | 3 edits | Info: "edited 3 times this step, consider reading full file" |
| Same file edited N times | 5 edits | Warning: "LOOP DETECTED" + full file contents + rollback path |
| File invalid after edit | 1 occurrence | Warning: show validation error + last valid snapshot path |
| Same error reappears after "fix" | 2 occurrences | Warning: "error reappeared, previous fix was incorrect" |
| Build error count increasing | 2 consecutive increases | Warning: "errors increasing (2 → 5), consider reverting" |

### Circuit Breaker Response

At the 5-edit threshold, the diagnostics inject a structured intervention:

```
LOOP DETECTED: config.json has been edited 5 times this step.
Invalid since command 5.

Current contents (full):
  {
    "assignment": "scan for vulnerabilities",
    "tools": ["run_command"
    "expected_output": "report findings"
  }

Validation error: Expected ',' or ']' at line 3, column 30

Last valid state (command 3): .snapshots/config.json@cmd-3
  Rollback: cp .snapshots/config.json@cmd-3 config.json

Recommendation: Read the full file, understand its current state,
and either rollback to the last valid version or rewrite entirely.
```

Three things the agent can't get on its own:
1. **Loop awareness** — it doesn't know it's been editing the same file 5 times
2. **Full current state** — it may have a stale mental model of the file
3. **Known-good rollback point** — it can undo to a working version in one command

### Error History Tracking

For build/test commands, track unique error messages:

```
Build 1: "error[E0308]: mismatched types at line 42"
Build 2: "error[E0599]: no method named 'foo' at line 78"  (new error, progress)
Build 3: "error[E0308]: mismatched types at line 42"  (ERROR REAPPEARED)
```

When an error reappears:
```
Warning: Error "mismatched types at line 42" reappeared after your fix.
Your change to line 42 may have been incorrect or reverted.
Error history:
  Build 1: mismatched types at line 42 (first seen)
  Build 2: no method 'foo' at line 78 (new error — your line 42 fix may have worked)
  Build 3: mismatched types at line 42 (REAPPEARED — fix was lost or wrong)
```

---

## Feature 4: Smart Output Truncation

Context-aware truncation that keeps the useful part of command output instead of blindly cutting at N bytes.

### The Problem

Current approach: truncate at first N bytes. But for the most common agent commands, the useful output is at the end:

- `npm install` — resolution tree at top (noise), errors and summary at bottom (signal)
- `cargo build` — "Compiling..." progress at top, actual error at bottom
- `pytest` — individual test output at top, pass/fail summary at bottom
- `pip install` — download progress at top, "Successfully installed" at bottom

SWE-Agent found empirically that ~100 lines is the optimal observation window. Too little context and the agent can't understand the output. Too much and it gets lost.

### Truncation Strategies

**Strategy 1: Tail-first for known verbose commands**

Detect command type, keep the last N lines instead of the first:

| Command Pattern | Strategy | Rationale |
|----------------|----------|-----------|
| `npm install`, `yarn install` | Last 30 lines | Summary + errors at bottom |
| `pip install` | Last 20 lines | "Successfully installed" at bottom |
| `cargo build`, `cargo check` | Last 50 lines | Error details at bottom |
| `cargo test`, `pytest`, `jest` | Parse (see below) | Only failures matter |
| `make` | Last 30 lines | Error at bottom |
| `docker build` | Last 30 lines | Final step + errors at bottom |
| Default | First 100 lines | Unknown commands, first output is usually relevant |

**Strategy 2: Structured parsing for test output**

Instead of truncating, parse test runner output and extract a structured summary:

```
$ cargo test

Full output: 2000 lines

Parsed summary:
  2231 passed, 3 failed, 0 ignored

  Failed tests:
    test_parse_config (src/config/tests.rs:42)
      AssertionError: expected "foo", got "bar"

    test_network_retry (src/network/tests.rs:118)
      TimeoutError: connection timed out after 5s

    test_edge_case (src/parser/tests.rs:203)
      assertion failed: expected 5, got 4
```

Supported test runners: `cargo test`, `pytest`, `jest`/`vitest`, `go test`. Each gets a parser that extracts pass/fail counts and failed test details.

**Strategy 3: Progress line stripping**

Remove lines that are purely progress indicators before truncation:

```
Compiling serde v1.0.163          ← strip (progress)
Compiling tokio v1.28.2           ← strip (progress)
Compiling my-app v0.1.0           ← keep (this is the user's crate)
error[E0308]: mismatched types     ← keep (error)
```

Patterns to strip:
- `Compiling <name> v<version>` (Rust) — keep only the final crate
- `Downloading <name>-<version>` (pip) — strip all
- `added N packages` lines (npm) — keep only the summary line
- Progress bars (`████░░░`, `[=====>   ]`, percentage indicators)
- ANSI escape sequences (`\033[...m`) — strip all before presenting to LLM

### Output Size Targets

Based on SWE-Agent's empirical findings, target ~100 lines of useful output per command. For commands that produce less, show everything. For commands that produce more, apply the appropriate strategy above.

Return the line count before and after truncation so the agent knows context was removed:

```
stdout (47 lines, showing last 30 of 892):
  ...
  Successfully installed flask-2.3.2 werkzeug-2.3.4
```

---

## Feature 5: Stderr Warning/Error Classification

Parse stderr output and classify each line so agents don't chase harmless warnings.

### The Problem

Agents see `WARN` or `WARNING` in stderr and panic. They start trying to fix deprecation warnings, upgrade pip versions, chase harmless noise — burning turns on things that don't affect correctness.

Real-world example from Aider's SWE-bench evaluation: 51.7% of edits had linting issues, many triggered by pre-existing warnings the agent tried to "fix."

### Classification Rules

**Error patterns** (likely cause of failure):
```
Error:, ERROR:, error:, error[E
FATAL:, fatal:
Traceback (most recent call last)
panic:, PANIC:
SyntaxError:, TypeError:, ValueError:, KeyError:, ImportError:
Cannot find module, Module not found
No such file or directory
Permission denied
command not found
Connection refused, Connection timed out
ENOENT, EACCES, ECONNREFUSED
```

**Warning patterns** (non-blocking, safe to ignore):
```
WARNING:, Warning:, warning:, WARN:
DeprecationWarning:, FutureWarning:, PendingDeprecationWarning:
npm WARN
pip WARNING: You are using pip version
UserWarning:
NOTE:, note:
```

**Info/noise patterns** (strip or minimize):
```
npm notice
Using pip version
Requirement already satisfied
up to date
```

### Output Format

Annotate stderr inline and add a summary:

```
stderr:
  [WARNING] npm WARN deprecated event-stream@3.3.4
  [WARNING] npm WARN deprecated uuid@3.4.0
  [ERROR]   Cannot find module 'express'

stderr summary: 2 warnings (non-blocking), 1 error
```

The summary line is the key — it gives the agent a single-glance signal. "2 warnings, 0 errors" means keep going. "0 warnings, 1 error" means stop and fix.

### Suggestion Rules (thefuck-style)

For common error patterns, append a fix suggestion:

| Error Pattern | Suggestion |
|---------------|------------|
| `command not found: <cmd>` | "Install with: apt-get install -y <package>" or "Not available in this container" |
| `No such file or directory: <path>` | "File doesn't exist. Check: ls <parent_dir>" |
| `Permission denied: <path>` | "Try: chmod +x <path>" |
| `Address already in use` | "Port in use. Kill: kill $(lsof -ti:<port>)" |
| `ModuleNotFoundError: <mod>` | "Install with: pip install <mod>" |
| `Cannot find module '<mod>'` | "Install with: npm install <mod>" |
| `database is locked` | "SQLite lock. Another process may have it open." |

These are pattern-matched from stderr using the same rule engine pattern as thefuck (match function + suggestion function). The rule set grows over time as we observe common agent failures.

---

## Feature 6: Workspace Digest

A one-line spatial awareness summary appended to every command response.

### The Problem

Agents lose track of what exists in the workspace. After 10 commands, they don't know how many files exist, what changed recently, or how large the workspace has grown. They run `ls` to re-orient — wasting a round trip.

### The Digest

After every command, append:

```
workspace: 14 files (+2), 3 dirs | last: reports/analysis.md | 12KB total
```

Components:
- **File count** with delta since last command: `14 files (+2)` or `14 files (-1)`
- **Directory count**: `3 dirs`
- **Last modified file**: the most recently changed file
- **Total size**: workspace size (excluding filtered junk)

### Extended Digest (on request or significant changes)

When the workspace changes significantly (5+ files created/modified), expand:

```
workspace: 23 files (+8), 5 dirs | 142KB total

  new:
    src/main.py (42 lines, Python)
    src/utils.py (18 lines, Python)
    src/models.py (65 lines, Python)
    tests/test_main.py (30 lines, Python)
    tests/test_utils.py (22 lines, Python)
    requirements.txt (3 lines)
    README.md (25 lines)
    setup.py (15 lines)

  modified:
    config.json (+2 fields)
```

### Workspace State Tracking

The workspace tracker maintains a lightweight index across commands:

```rust
struct WorkspaceState {
    files: HashMap<PathBuf, FileMetadata>,
    total_size: u64,
    command_index: usize,
}

struct FileMetadata {
    size: u64,
    line_count: Option<usize>,  // None for binary files
    file_type: FileType,
    last_modified_at_command: usize,
}
```

Updated after each command from the OverlayFS diff — not a full filesystem walk. Only changed files are re-indexed.

---

## The Response Envelope

Every `run_command` invocation returns a `CommandEnvelope`:

```
result: success | success (no-op) | partial | failed | timeout

pre-execution warnings: (if any)
  cd doesn't persist between commands. Chain with &&.

stdout (3 lines):
  Processing 5 files...
  Analysis complete.
  Report written.

stderr summary: 2 warnings (non-blocking), 0 errors

workspace changes:
  created: reports/analysis.md (84 lines, Markdown)
  created: reports/charts/pricing.png (142KB, image)
  modified: config.json
    .version: "1.0" → "2.0"
    .modules: added ["main", "utils"]
    validation: valid JSON
  unchanged: 11 files

loop status: clean (no repeated edits)

workspace: 16 files (+2), 4 dirs | last: reports/analysis.md | 89KB total

rollback: .snapshots/cmd-7
```

### Envelope Severity

The envelope carries a top-level severity that the agent can use for routing:

| Severity | Meaning | Agent should... |
|----------|---------|-----------------|
| `ok` | Command worked, changes as expected | Continue with task |
| `info` | Worked, with observations | Note the observations, continue |
| `no-op` | Command succeeded but did nothing | Check assumptions, retry with corrected input |
| `warning` | Likely problem detected | Address the warning before continuing |
| `error` | Command failed | Investigate error, fix, retry |
| `loop` | Repeated edit pattern detected | Step back, read full file, consider rollback or rewrite |

The severity is the maximum across all diagnostics in the envelope. If any post-check returns `error`, the envelope is `error` even if the exit code was 0.

---

## Module Architecture

### File Tree

```
src/execution/diagnostics/
├── mod.rs                     # DiagnosticsEngine — orchestrates the pipeline
├── envelope.rs                # CommandEnvelope, Severity, Diagnostic types
├── types.rs                   # FileChange, FileType, shared types
│
├── pre/
│   ├── mod.rs                 # PreCheck trait + pre-check runner
│   ├── state_persistence.rs   # cd, export, alias persistence warnings
│   ├── interactive.rs         # REPL/prompt hang detection
│   ├── shell_compat.rs        # bash vs sh syntax warnings
│   └── tests.rs
│
├── post/
│   ├── mod.rs                 # PostCheck trait + post-check runner
│   ├── noop.rs                # No-op detection for mutation/search commands
│   ├── nearest_match.rs       # Fuzzy pattern search for no-op suggestions
│   ├── stderr_classifier.rs   # Warning vs error line classification
│   ├── truncation.rs          # Smart tail-first output truncation
│   ├── suggestions.rs         # thefuck-style fix suggestions for common errors
│   └── tests.rs
│
├── workspace/
│   ├── mod.rs                 # WorkspaceTracker — state across commands
│   ├── digest.rs              # Post-command workspace summary
│   ├── snapshot.rs            # Pre-command upper dir checkpoint
│   ├── rollback.rs            # Restore from checkpoint
│   ├── validation.rs          # Format-aware file validation (JSON, Python, YAML)
│   └── tests.rs
│
├── loop_detector/
│   ├── mod.rs                 # Edit history tracking + circuit breaker
│   └── tests.rs
│
└── diff/
    ├── mod.rs                 # Diff dispatch by file type
    ├── json_diff.rs           # Field-level JSON structural diffs
    ├── line_diff.rs           # Standard line-level diffs (fallback)
    └── tests.rs
```

### Core Traits

```rust
// pre/mod.rs — runs before command execution
trait PreCheck: Send + Sync {
    /// Analyze the command string and return a diagnostic if a known
    /// problem is detected. Return None if the command looks fine.
    fn check(&self, command: &str) -> Option<Diagnostic>;
}

// post/mod.rs — runs after command execution
trait PostCheck: Send + Sync {
    /// Analyze the command, its result, and filesystem changes.
    /// Return zero or more diagnostics.
    fn check(
        &self,
        command: &str,
        result: &ExecResult,
        changes: &[FileChange],
    ) -> Vec<Diagnostic>;
}
```

Each feature is a struct implementing one of these traits. The engine holds a `Vec<Box<dyn PreCheck>>` and `Vec<Box<dyn PostCheck>>`, iterates through them, and collects diagnostics into the envelope.

### Core Types

```rust
// envelope.rs
struct CommandEnvelope {
    command: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration: Duration,
    severity: Severity,
    pre_warnings: Vec<Diagnostic>,
    post_diagnostics: Vec<Diagnostic>,
    file_changes: Vec<FileChange>,
    loop_status: LoopStatus,
    workspace_digest: WorkspaceDigest,
    rollback_path: Option<PathBuf>,
}

enum Severity {
    Ok,
    Info,
    NoOp,
    Warning,
    Error,
    Loop,
}

struct Diagnostic {
    severity: Severity,
    category: DiagnosticCategory,
    message: String,
    suggestion: Option<String>,
}

enum DiagnosticCategory {
    StatePersistence,
    InteractiveCommand,
    ShellCompat,
    NoOp,
    NearestMatch,
    StderrClassification,
    Truncation,
    Validation,
    LoopDetected,
    ErrorReappeared,
    Suggestion,
}
```

```rust
// types.rs
struct FileChange {
    path: PathBuf,
    change_type: ChangeType,
    file_type: FileType,
    size: u64,
    line_count: Option<usize>,
    diff: Option<FileDiff>,
    validation: Option<ValidationResult>,
}

enum ChangeType {
    Created,
    Modified,
    Deleted,
}

enum FileDiff {
    Json(JsonDiff),
    Lines(LineDiff),
    Binary { old_size: u64, new_size: u64 },
}

struct JsonDiff {
    added: Vec<JsonPath>,
    removed: Vec<JsonPath>,
    changed: Vec<(JsonPath, Value, Value)>,  // path, old, new
}

struct LineDiff {
    hunks: Vec<DiffHunk>,
    lines_added: usize,
    lines_removed: usize,
}

enum ValidationResult {
    Valid,
    Invalid { error: String, line: Option<usize>, column: Option<usize> },
}
```

### Engine Orchestration

```rust
// mod.rs
struct DiagnosticsEngine {
    pre_checks: Vec<Box<dyn PreCheck>>,
    post_checks: Vec<Box<dyn PostCheck>>,
    workspace: WorkspaceTracker,
    loop_detector: LoopDetector,
    command_index: usize,
}

impl DiagnosticsEngine {
    fn new() -> Self {
        Self {
            pre_checks: vec![
                Box::new(StatePersistenceCheck),
                Box::new(InteractiveCheck),
                Box::new(ShellCompatCheck),
            ],
            post_checks: vec![
                Box::new(NoOpCheck),
                Box::new(NearestMatchCheck),
                Box::new(StderrClassifier),
                Box::new(SmartTruncation),
                Box::new(SuggestionEngine),
            ],
            workspace: WorkspaceTracker::new(),
            loop_detector: LoopDetector::new(),
            command_index: 0,
        }
    }

    fn execute(&mut self, command: &str, container: &Container) -> CommandEnvelope {
        self.command_index += 1;

        // Phase 1: Pre-checks
        let pre_warnings: Vec<Diagnostic> = self.pre_checks
            .iter()
            .filter_map(|check| check.check(command))
            .collect();

        // Phase 2: Snapshot + Execute + Diff
        let snapshot = self.workspace.snapshot(container);
        let result = container.exec(command);
        let changes = self.workspace.diff(container, &snapshot);

        // Phase 3: Post-checks
        let post_diagnostics: Vec<Diagnostic> = self.post_checks
            .iter()
            .flat_map(|check| check.check(command, &result, &changes))
            .collect();

        // Phase 4: Stateful tracking
        let loop_status = self.loop_detector.record(
            self.command_index, &changes
        );
        let digest = self.workspace.digest();

        // Phase 5: Assemble envelope
        let severity = Self::max_severity(
            &pre_warnings, &post_diagnostics, &loop_status
        );

        CommandEnvelope {
            command: command.to_string(),
            exit_code: result.exit_code,
            stdout: result.stdout,
            stderr: result.stderr,
            duration: result.duration,
            severity,
            pre_warnings,
            post_diagnostics,
            file_changes: changes,
            loop_status,
            workspace_digest: digest,
            rollback_path: snapshot.checkpoint_path(),
        }
    }
}
```

### Rendering the Envelope for the LLM

The envelope is a Rust struct, but the agent sees text. A renderer converts it:

```rust
impl CommandEnvelope {
    fn render(&self) -> String {
        // Renders the structured envelope into the text format
        // shown in the examples above. This is what goes into
        // the agent's tool response.
    }
}
```

The renderer is the boundary between structured data (for programmatic use, logging, metrics) and text (for the LLM). The system stores the full envelope; the agent sees the rendered text.

---

## What This Builds On

| Existing | Diagnostics adds |
|----------|-----------------|
| `ContainerHandle.exec_shell()` | Wraps with snapshot/diff/analyze pipeline |
| `ContainerExecResult` (stdout, stderr, exit_code) | `CommandEnvelope` with diagnostics, changes, digest |
| OverlayFS upper dir walk (`extract_overlay_diff`) | Per-command checkpoints and workspace tracking |
| `classify.rs` file type detection | Format-aware validation and structural diffs |
| `CONTAINER_MAX_OUTPUT_BYTES` truncation | Smart tail-first truncation by command type |
| `CONTAINER_COMMAND_TIMEOUT_SECS` | Pre-execution detection of commands that will hang |

## What This Doesn't Replace

- **Container execution** — the diagnostics engine wraps it, doesn't replace it
- **OverlayFS denylist filtering** — still runs at step completion to filter junk before JuiceFS persistence. Diagnostics operates per-command during the step.
- **Agent tool definitions** — `run_command` remains the tool schema the LLM sees. The diagnostics engine enriches the tool response.
- **Step-level handoffs** — the envelope is per-command feedback to the agent. The step handoff (expected_output) is the agent's final text output for the next step.

## Research References

- [SWE-Agent ACI](https://arxiv.org/abs/2405.15793) — Agent-Computer Interface design, edit-time linting, ~100 line observation window
- [OpenHands](https://arxiv.org/abs/2407.16741) — Event-sourced architecture, typed observations
- [AgentRefine](https://arxiv.org/abs/2501.01702) — Agent failure taxonomy, "stuck in same mistake" pattern
- [BuildKit OverlayFS diff](https://github.com/moby/buildkit/pull/2181) — Walking upper dir directly instead of full filesystem compare
- [overlayfs-tools](https://github.com/kmxz/overlayfs-tools) — diff, vacuum, merge for OverlayFS upper directories
- [Warp Terminal Blocks](https://www.warp.dev/blog/the-data-structure-behind-terminals) — Structured command execution blocks
- [Difftastic](https://github.com/Wilfred/difftastic) — AST-aware structural diffs via tree-sitter
- [thefuck](https://github.com/nvbn/thefuck) — Rule-based error pattern matching and fix suggestions
- [Nushell](https://www.nushell.sh/) — Structured/typed command output
- [json-patch (RFC 6902)](https://github.com/idank/json-patch) — Structural JSON diff operations
- [Graphtage](https://github.com/trailofbits/graphtage) — Cross-format semantic tree diffs
- [content_inspector](https://docs.rs/content_inspector/) — Rust binary-vs-text file detection
- [Google Magika](https://github.com/google/magika) — AI-based file type detection
