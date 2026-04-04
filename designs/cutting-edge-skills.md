# Cutting-Edge Claude Code Skills

Research-backed skill designs that encode **techniques**, not prompts. Each skill gives Claude a specific algorithm, decomposition strategy, or invariant to check — something that constrains the search space and produces structured, verifiable findings.

The distinction: a prompt says "find bugs." A technique says "extract every enum that represents a state, build the transition table from all match arms, report states with no inbound transitions." The technique is reproducible, auditable, and leverages Claude's reasoning on a focused problem instead of asking it to boil the ocean.

---

## Table of Contents

1. [Co-Change Coupling](#1-co-change-coupling) — Git history as a bug detector
2. [Async Audit](#2-async-audit) — Rust async footguns that compile clean
3. [State Machine Extraction](#3-state-machine-extraction) — Find impossible states in implicit machines
4. [Doc-Code Drift](#4-doc-code-drift) — Catch docs that lie
5. [Clone Divergence](#5-clone-divergence) — One copy got fixed, the other didn't
6. [Boundary Interrogation](#6-boundary-interrogation) — Off-by-one and empty-input analysis
7. [Phantom Type Gaps](#7-phantom-type-gaps) — Where the type system is satisfied but semantics are wrong
8. [Error Path Consistency](#8-error-path-consistency) — State left broken after early returns
9. [Git Archaeology](#9-git-archaeology) — Reconstruct WHY code exists before changing it
10. [Dead Reference Scanner](#10-dead-reference-scanner) — Find doc references to deleted code

---

## 1. Co-Change Coupling

**What it does:** Uses git history to find files that historically change together but weren't *both* changed in this PR. Also identifies bug hotspots using Google's bugspots algorithm (recency-weighted fix frequency).

**Why it's not just a prompt:** It encodes two specific algorithms:
- **Bugspots scoring:** `score = sum(1/(1 + e^(-12*t + 12)))` for each bug-fixing commit, where `t` is normalized time. Files in the top 10% are hotspots. This is Google's actual algorithm (igrigorik/bugspots).
- **Co-change association rules:** From Zimmermann et al. (IEEE TSE 2005) — association rule mining on version histories. Their top 3 suggestions contained a correct "you also need to change this" location >70% of the time.

Adam Tornhill's "Your Code as a Crime Scene" showed that in a 400KLOC codebase, hotspots pinpointed 7 of 8 most defect-dense areas, and 4% of the code was responsible for 72% of all defects.

**Sources:**
- [Bugspots — Google's algorithm](https://github.com/igrigorik/bugspots)
- [ROSE: Mining Version Histories (IEEE TSE 2005)](https://thomas-zimmermann.com/publications/files/zimmermann-tse-2005.pdf)
- [Code-Maat](https://github.com/adamtornhill/code-maat)
- [Your Code as a Crime Scene (Tornhill)](https://pragprog.com/titles/atcrime/your-code-as-a-crime-scene/)

### SKILL.md

```yaml
---
name: co-change
description: Find historically co-changed files missing from this PR and bug hotspots using git history algorithms
disable-model-invocation: true
argument-hint: [branch-name or leave blank for current]
allowed-tools: Bash Read Grep Glob
---

Analyze git history to find **co-change coupling violations** and **bug hotspots**.

## Step 1: Identify files in this change

Get the files changed on this branch vs main:

```
git diff --name-only main...HEAD
```

## Step 2: Build co-change frequency matrix

For each changed file, find which OTHER files historically change in the same commit:

```
# For each file in the PR, find its co-change partners
git log --pretty=format:"%H" -- <file> | while read hash; do
  git diff-tree --no-commit-id --name-only -r "$hash"
done | sort | uniq -c | sort -rn
```

A file pair with >60% co-change rate is **coupled**. If one is in the PR and the other isn't, that's a potential incomplete change.

## Step 3: Compute bugspots scores

Find bug-fixing commits (messages matching fix/bug/patch/resolve/close):

```
git log --format="%H %at %s" --all | grep -iE "(fix|bug|patch|resolve|close)"
```

For each file touched by fix commits, compute:
- `t` = normalized timestamp (0 = oldest fix, 1 = most recent)
- `score += 1/(1 + e^(-12*t + 12))`

Recent fixes weight exponentially more than old ones.

## Step 4: Cross-reference

For the files in this PR:
1. **Coupling violations**: Files with >60% co-change rate that are NOT in this PR. Show the co-change rate and recent commits where they changed together.
2. **Hotspot overlap**: If any PR file is in the top 10% of hotspots, flag it as high-risk and recommend extra review.
3. **Divergent fix detection**: Find file pairs with >80% co-change rate where the most recent commit changed only ONE of the pair. Show what changed — the other file may need the same fix.

## Output format

### Co-Change Violations
| Missing File | Co-Changed With | Rate | Last Joint Commit |
|---|---|---|---|

### Bug Hotspots in This PR
| File | Score | Fix Commits (last 6mo) | Risk |
|---|---|---|---|

### Divergent Fixes (one got fixed, other didn't)
| Fixed File | Unfixed Partner | Fix Commit | What Changed |
|---|---|---|---|

Only report findings with evidence. No speculative warnings.
```

---

## 2. Async Audit

**What it does:** Scans Rust async code for specific footgun patterns that compile clean but cause deadlocks, resource starvation, or data corruption at runtime.

**Why it's not just a prompt:** Each check is a specific, documented pattern with a known failure mode:
- `MutexGuard` held across `.await` → deadlock (Tokio reschedules to another thread)
- `std::sync::Mutex` in async function where guard spans await → same deadlock
- Blocking I/O in async context without `spawn_blocking` → thread pool starvation
- Futures recreated inside `select!` loops → allocation waste, message loss
- Missing cancellation safety → cleanup code after `.await` never runs

These aren't opinions — they're documented failure modes from Cloudflare outages, Turso's deadlock writeup, and Tokio's own docs.

**Sources:**
- [Common Mistakes with Rust Async (Qovery)](https://www.qovery.com/blog/common-mistakes-with-rust-async)
- [How to Deadlock Tokio (Turso)](https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex)
- [Tokio Shared State Tutorial](https://tokio.rs/tokio/tutorial/shared-state)
- [Best and Worst Deadlock in Rust (Snoyman)](https://www.snoyman.com/blog/2024/01/best-worst-deadlock-rust/)

### SKILL.md

```yaml
---
name: audit-async
description: Scan Rust async code for deadlocks, resource starvation, and cancellation-safety bugs
disable-model-invocation: true
argument-hint: [file-or-module path, or blank for whole src/]
allowed-tools: Bash Read Grep Glob
---

Audit Rust async code for specific documented footgun patterns. Target: $ARGUMENTS (default: entire src/).

## Check 1: Mutex guards held across .await

Find every `lock()`, `read()`, `write()` call on Mutex/RwLock types. Trace the guard's scope. If the scope contains an `.await`, flag it.

**Why it's a bug:** Tokio may suspend the task and resume it on a different thread. `std::sync::MutexGuard` is not `Send`, causing a deadlock. Even `tokio::sync::MutexGuard` held across await points can cause contention starvation.

**Fix:** Scope the guard so it drops before the `.await`, or use `tokio::sync::Mutex` if the guard must span the await (with a comment explaining why).

## Check 2: std::sync::Mutex in async functions

Grep for `std::sync::Mutex` or `parking_lot::Mutex` usage in `async fn` or functions returning `impl Future`. If the guard is held across any `.await`, it MUST be `tokio::sync::Mutex`.

Conversely, if `tokio::sync::Mutex` is used but no `.await` exists within the guard scope, flag it as unnecessary overhead — `std::sync::Mutex` is cheaper.

## Check 3: Blocking operations in async context

Search for these patterns inside `async fn`:
- `std::fs::` operations (use `tokio::fs::` instead)
- `std::thread::sleep` (use `tokio::time::sleep`)
- `std::net::` (use `tokio::net::`)
- `.read()` / `.write()` on `std::io` types
- CPU-heavy loops without `spawn_blocking`

**Why it's a bug:** Blocks a Tokio worker thread. With the default 4-thread pool, 4 concurrent blocking calls freeze the entire runtime.

## Check 4: Futures recreated inside select! loops

Find `loop { select! { ... } }` patterns. Check if any branch creates a new future on each iteration (e.g., `some_async_fn()` called inline). The future should be created outside the loop with `pin!()`.

**Why it's a bug:** Each iteration allocates a new future, discarding partial progress. For channel receives or timers, this means dropped messages.

## Check 5: Cancellation safety

Find any `.await` where the code after it performs cleanup (state mutation, file deletion, channel sends). If the task is cancelled at that await point, the cleanup never runs.

**Pattern to flag:**
```rust
state.mark_started();
let result = do_work().await;  // <-- cancellation here means...
state.mark_finished();          // <-- ...this never runs
```

**Fix:** Use RAII guards (`scopeguard::defer!`) or `tokio::select!` with cancellation-safe branches.

## Check 6: Blocking Drop implementations

Find `impl Drop for` on types used in async code. If the `drop()` method does I/O (file ops, network, database), it blocks the runtime.

## Output

For each finding, report:
- **File:line** — exact location
- **Pattern** — which check triggered
- **Severity** — Deadlock (critical), Starvation (high), Inefficiency (medium)
- **Fix** — specific code change, not generic advice

Group by severity. Skip anything behind `#[cfg(test)]`.
```

---

## 3. State Machine Extraction

**What it does:** Finds implicit state machines in code (enums representing states, status fields), extracts the full transition table from all match/if-let arms, and identifies dead states, impossible transitions, and missing handlers.

**Why it's not just a prompt:** It's a formal procedure from SMBugFinder (ISSTA 2024): extract states → enumerate transitions → check properties. The properties are specific: unreachable states (no inbound transitions), terminal states (no outbound — intentional?), wildcard arms hiding new variants. This is automata theory applied to code review.

The Scott Logic blog documents that invalid-state bugs are "easy to introduce and difficult to detect in large, complex codebases" because the state machine is implicit in scattered if/else chains. Making it explicit is the technique.

**Sources:**
- [SMBugFinder (ISSTA 2024)](https://github.com/assist-project/state-machine-bug-finder)
- [Finite State Machines: The Developer's Bug Spray (Scott Logic)](https://blog.scottlogic.com/2020/09/22/finite-state-machines-the-developers-bug-spray.html)

### SKILL.md

```yaml
---
name: state-machine
description: Extract implicit state machines from enums/status fields, build transition tables, find dead states
disable-model-invocation: true
argument-hint: [enum-name or module-path]
allowed-tools: Bash Read Grep Glob
---

Extract the implicit state machine for $ARGUMENTS and audit it.

## Step 1: Identify state types

Find all enums that represent states, statuses, or phases. Indicators:
- Name contains `State`, `Status`, `Phase`, `Stage`, `Mode`, `Step`
- Variants named like lifecycle stages: `Pending`, `Running`, `Complete`, `Failed`
- Used in match arms that transition between variants

If $ARGUMENTS names a specific enum, use that. Otherwise scan the target module.

## Step 2: Build the transition table

For each state enum, search the ENTIRE codebase for every `match` or `if let` on that type. For each arm, record:
- **From state**: The matched variant
- **To state**: Any assignment to the same type within that arm
- **Trigger**: What causes this transition (function name, message type, condition)
- **Location**: file:line

Build a table:
| From | To | Trigger | Location |
|---|---|---|---|

## Step 3: Check automata properties

**3a. Unreachable states:** Variants with NO inbound transitions (never transitioned TO). These are either initial states or dead code.

**3b. Terminal states:** Variants with NO outbound transitions (never transitioned FROM). Verify these are intentionally terminal (e.g., `Completed`, `Failed`). If not, it's a state that traps execution.

**3c. Wildcard absorption:** Any `_ => ...` or `other => ...` match arm that handles unknown variants. These silently absorb new variants added later. Flag with: "Adding a new variant to this enum will be silently handled by the wildcard at file:line instead of forcing explicit handling."

**3d. Missing transitions:** For each (from, trigger) pair, check if there's an explicit handler. If state A handles triggers X, Y, Z, but state B only handles X, Y — trigger Z in state B is either impossible (good) or unhandled (bug).

**3e. Contradictory transitions:** Same (from, trigger) producing different to-states in different locations.

## Step 4: Draw the machine

Output an ASCII state diagram:

```
[Pending] --start--> [Running] --complete--> [Done]
                         |
                       --fail--> [Failed]
```

## Output

1. The transition table
2. The state diagram
3. Findings: dead states, trapping states, wildcard absorption, missing handlers
4. For each finding: the exact location and whether it's a bug or intentional
```

---

## 4. Doc-Code Drift

**What it does:** Scans all documentation for references to code elements (function names, file paths, CLI commands, config keys) and verifies each reference against the actual codebase. Uses git blame to compute staleness scores.

**Why it's not just a prompt:** It combines three concrete techniques:
1. **Tan-Wagner-Treude reference checking** (ICSE 2024): 28.9% of top-1000 GitHub projects contain outdated code element references. Their algorithm: extract backticked identifiers → check against source tree → flag missing.
2. **IBM staleness scoring** (US Patent 8,607,193): staleness proportional to code change extent, developer identity mismatch, and structural changes.
3. **Spec-gen gap detection**: map doc sections to code files, flag docs whose associated code changed but the doc didn't.

**Sources:**
- [Detecting Outdated Code Element References (Springer/ICSE 2024)](https://link.springer.com/article/10.1007/s10664-023-10397-6)
- [IBM Patent US8607193 — Staleness Scoring](https://patents.google.com/patent/US8607193B2/en)
- [spec-gen](https://github.com/clay-good/spec-gen)
- [Swimm Auto-sync](https://docs.swimm.io/features/keep-docs-updated-with-auto-sync/)

### SKILL.md

```yaml
---
name: doc-drift
description: Find documentation references to deleted code, stale commands, and drifted descriptions
disable-model-invocation: true
argument-hint: [doc-file-or-directory, or blank for all .md files]
allowed-tools: Bash Read Grep Glob
---

Scan documentation for references that have drifted from reality. Target: $ARGUMENTS (default: all .md files in the repo).

## Phase 1: Extract references

For each markdown file, extract:

**1a. Backticked identifiers** — anything in single backticks that looks like a code symbol: function names (`snake_case`), type names (`CamelCase`), constants (`UPPER_CASE`), enum variants (`Type::Variant`).

**1b. File paths** — anything matching `src/`, `frontend/`, `config/`, or other project directory patterns. Also relative paths like `./foo/bar.rs`.

**1c. Shell commands** — anything in code blocks tagged as `bash`, `sh`, `shell`, or starting with `$`, `cargo`, `npm`, `npx`, `docker`. Extract the command and its arguments.

**1d. Config keys** — environment variable names (`UPPER_SNAKE`), TOML/YAML keys, CLI flags.

## Phase 2: Verify each reference

**2a. Code symbols:** Search the codebase with grep. If no match, check git log to confirm it once existed (deletion vs. typo). Report:
- DELETED: existed in git history, now removed
- NEVER_EXISTED: no git history match (likely a typo or pseudo-code)
- RENAMED: similar symbol exists (Levenshtein distance < 3)

**2b. File paths:** Check if the path exists. If not, check git log for the old path. Report:
- DELETED: file was removed
- MOVED: similar filename exists elsewhere
- NEVER_EXISTED: no history

**2c. Shell commands:** Verify:
- Referenced binaries exist (`which` or `command -v`)
- Referenced cargo modules/tests exist (`grep` for the module path)
- Flags are valid for the command version

**2d. Config keys:** Search for usage in code. If the key is documented but never read by any code, flag it.

## Phase 3: Staleness scoring

For each doc file, compare:
- `doc_age`: last modified date of the doc (via git blame)
- `code_age`: last modified date of the code it references
- `drift_score`: number of code commits since the doc was last touched, weighted by change size

Flag docs where drift_score > 10 (the code has changed significantly since the doc was written).

## Phase 4: Command verification

For shell commands in CLAUDE.md, README.md, and any setup/installation docs:
- Verify the commands actually work (check paths, module names, flags exist)
- Check if the output format described in docs matches current behavior

## Output

### Dead References
| Doc File | Line | Reference | Status | Last Existed |
|---|---|---|---|---|

### Stale Documentation
| Doc File | Last Updated | Code Changes Since | Drift Score |
|---|---|---|---|

### Broken Commands
| Doc File | Line | Command | Issue |
|---|---|---|---|

Only report confirmed issues. Do NOT flag intentional pseudo-code or placeholder examples.
```

---

## 5. Clone Divergence

**What it does:** Given a recent bug fix, extracts the abstract pattern of the bug, then searches the codebase for structurally similar code that might have the same unfixed bug.

**Why it's not just a prompt:** This is the BugStone pipeline (arXiv 2510.14036) adapted for a code assistant. BugStone identified 22K+ potential issues in the Linux kernel from 135 seed bugs with 92.2% precision. The key insight: converting a patch into an abstract "security coding rule" and checking candidates against that rule is radically more precise than asking "find bugs."

KNighter (SOSP 2025) took this further — synthesizing actual static analyzers from patches. Found 92 new Linux kernel bugs (avg age 4.3 years), 57 fixed, 30 assigned CVEs.

The technique decomposes into: extract rule → enumerate candidates → evaluate each. Three steps, each focused.

**Sources:**
- [BugStone (arXiv 2510.14036)](https://arxiv.org/html/2510.14036v1)
- [KNighter (SOSP 2025)](https://arxiv.org/html/2503.09002v2)
- [Kill the Clones (Tornhill)](https://www.adamtornhill.com/articles/aspnetclones/killtheclones.html)

### SKILL.md

```yaml
---
name: clone-divergence
description: Given a bug fix, find other code locations with the same unfixed pattern
disable-model-invocation: true
argument-hint: [commit-hash or file:line of the fix]
allowed-tools: Bash Read Grep Glob
---

Given a bug fix at $ARGUMENTS, find other locations in the codebase that may have the same unfixed bug.

## Step 1: Extract the fix pattern

Read the diff of the fix (commit or staged change). Identify:
- **Pre-pattern**: What the buggy code looked like (the `-` lines in context)
- **Post-pattern**: What the fixed code looks like (the `+` lines)
- **Abstract rule**: The general principle — e.g., "must check for empty vec before indexing" or "must hold lock before reading shared state" or "must validate input length before slicing"

State the rule in one sentence. This is the search target.

## Step 2: Build search queries

From the pre-pattern, extract structural markers — function calls, method chains, type names, variable patterns — that would appear in code with the same bug. Build 2-3 grep patterns that would match structurally similar code.

Example: if the fix was adding `.unwrap_or_default()` after `.get()`, search for all `.get(` calls that don't have null-handling.

## Step 3: Enumerate candidates

Search the codebase for all matches. Exclude:
- The already-fixed location
- Test files (unless the bug is in test infrastructure)
- Generated code

## Step 4: Evaluate each candidate

For each candidate, assess:
- **VULNERABLE**: The pre-pattern matches and the fix is absent. This is likely the same bug.
- **SAFE**: The code has the same structure but already handles the case (different fix, same effect).
- **DIFFERENT**: Superficially similar but the context makes the bug impossible here.

## Output

### Bug Pattern
**Rule:** [one-sentence abstract rule]
**Fixed at:** [location]
**Fix:** [what changed]

### Unfixed Clones
| Location | Code Snippet | Assessment | Confidence |
|---|---|---|---|

### Safe Clones (already handled differently)
| Location | How It's Handled |
|---|---|

Report ONLY locations you've actually read and evaluated. Never guess from grep output alone.
```

---

## 6. Boundary Interrogation

**What it does:** For every loop, index operation, slice, range, or collection access in a target function/file, systematically checks the boundary conditions: empty input, single element, off-by-one on indices, and `<` vs `<=` correctness.

**Why it's not just a prompt:** Boundary Value Analysis has "disproportionate defect-detection power compared to interior values" — a single test at a boundary is worth more than dozens of tests in the middle. The skill encodes a specific checklist (5 questions per access pattern) applied mechanically to every relevant code site. This is the technique security auditors use, just systematized.

The UT Austin paper "Static Detection of Asymptotic Performance Bugs in Collection Traversals" (PLDI 2015) showed that collection traversal patterns have a small, checkable set of boundary conditions. The skill exploits this finite set.

**Sources:**
- [Boundary Value Analysis (Ryan Craven)](https://ryancraventech.substack.com/p/boundary-value-analysis-finding-bugs)
- [Static Detection of Performance Bugs in Collection Traversals (PLDI 2015)](https://dl.acm.org/doi/10.1145/2737924.2737966)

### SKILL.md

```yaml
---
name: boundary
description: Systematically check every loop, index, and range for off-by-one and empty-input bugs
disable-model-invocation: true
argument-hint: [file-path or function-name]
allowed-tools: Read Grep Glob
---

Interrogate every boundary condition in $ARGUMENTS.

## For each loop (`for`, `while`, iterator chain):

1. **Empty collection**: What happens if the input is empty? Does the code produce the correct result (empty output, zero, identity) or does it panic/produce garbage?
2. **Single element**: Does the logic work correctly with exactly one item? (Many off-by-one bugs only manifest with 1 or 2 elements.)
3. **Bound correctness**: Is the range `0..n` or `0..=n`? Should it be the other? Check what happens at `n-1`, `n`, and `n+1`.
4. **First/last special cases**: Does the loop body assume it's not on the first or last iteration? (e.g., accessing `i-1` without checking `i > 0`)
5. **Early termination**: If the loop uses `break` or `return`, what's the state of any accumulated result?

## For each index/slice operation (`[i]`, `[start..end]`, `.get(i)`):

1. **Could `i` equal the length?** `vec[vec.len()]` panics. `vec[vec.len() - 1]` panics on empty vec.
2. **Could `i` be negative (or wrap)?** In Rust, `usize` subtraction wraps. `0usize - 1` is `usize::MAX`.
3. **Slice bounds**: `[start..end]` — could `start > end`? Could `end > len`?
4. **Is `.get()` used where `[]` is used?** If the index comes from external input or computation, `.get()` with error handling is safer.

## For each arithmetic operation producing an index or size:

1. **Division**: Could the divisor be zero?
2. **Subtraction on unsigned**: Could this wrap? (`a - b` where `a < b` and both are `usize`)
3. **Multiplication**: Could this overflow?
4. **Integer division rounding**: `5 / 2 = 2` in integer math. Is truncation correct or should it round up?

## Output format

For each finding:
- **Location**: file:line
- **Pattern**: which check triggered
- **Question**: the specific boundary condition that's suspect
- **Verdict**: BUG (will fail), SUSPECT (could fail with certain inputs), or SAFE (handled correctly — explain how)

Only report BUG and SUSPECT findings. Don't list SAFE items unless they use a non-obvious technique worth noting.
```

---

## 7. Phantom Type Gaps

**What it does:** Finds places where primitive types (`i64`, `String`, `f64`) are used for semantically distinct values — user IDs vs group IDs, pixels vs meters, seconds vs milliseconds — where the type system is satisfied but a mixup would be a silent logic bug.

**Why it's not just a prompt:** The Mars Climate Orbiter ($125M) was lost because one subsystem used pound-force-seconds and another used newton-seconds — same numeric type, different semantics. TypePulse (USENIX Security 2025) detects type confusion bugs in Rust programs specifically.

The technique: infer semantic domains from naming, then check if values from different domains are mixed in operations. This is a specific inference + checking procedure, not a vague "review types."

**Sources:**
- [TypePulse (USENIX Security 2025)](https://arxiv.org/html/2502.03271v1)
- [Mars Climate Orbiter (SimScale)](https://www.simscale.com/blog/nasa-mars-climate-orbiter-metric/)
- [Newtypes in Rust](https://doc.rust-lang.org/rust-by-example/generics/new_types.html)

### SKILL.md

```yaml
---
name: phantom-types
description: Find where primitive types mask semantic domain mismatches (wrong ID, wrong unit, wrong coordinate)
disable-model-invocation: true
argument-hint: [module-path or blank for full scan]
allowed-tools: Read Grep Glob
---

Find semantic type confusion risks in $ARGUMENTS.

## Step 1: Catalog primitive usage

Find all function signatures and struct definitions. For each parameter or field that uses a primitive type (`i64`, `i32`, `u64`, `u32`, `usize`, `f64`, `f32`, `String`, `&str`), record:
- The name
- The inferred semantic domain (from naming: `_id`, `_count`, `_ms`, `_seconds`, `_bytes`, `_px`, `_index`, `_offset`, `_price`, `_score`)

## Step 2: Find confusion-prone signatures

**2a. Multiple same-type parameters from different domains:**
```rust
fn transfer(from_id: i64, to_id: i64, amount: i64)  // 3 i64s, 2 domains
```
Flag any function with 2+ parameters of the same primitive type where the names suggest different semantic domains.

**2b. Cross-domain arithmetic:**
```rust
let total = user_count + item_count;  // both usize, but adding apples and oranges
```
Flag arithmetic operations where the operand names suggest different domains.

**2c. Cross-domain assignment:**
```rust
let user_id = group_id;  // both i64, but semantically wrong
```
Flag assignments where the variable names suggest a domain mismatch.

## Step 3: Recommend newtypes

For each semantic domain that appears 3+ times, suggest a newtype wrapper:
```rust
struct UserId(i64);
struct GroupId(i64);
struct Milliseconds(u64);
```

Estimate the blast radius: how many function signatures and struct fields would need to change.

## Output

### High Risk: Confusion-Prone Signatures
| Function | Parameters | Domains | Risk |
|---|---|---|---|

### Medium Risk: Cross-Domain Operations
| Location | Operation | Left Domain | Right Domain |
|---|---|---|---|

### Recommended Newtypes
| Domain | Current Type | Occurrences | Blast Radius |
|---|---|---|---|

Skip types that are already wrapped (existing newtypes). Focus on the highest-risk domain mixups.
```

---

## 8. Error Path Consistency

**What it does:** For every function that modifies state and can fail (returns `Result`), traces what happens when failure occurs mid-operation. Checks if partial state mutations are rolled back or if the system is left inconsistent.

**Why it's not just a prompt:** Approximately 20% of errors never make it to logs (Harness blog on swallowed exceptions). But the subtler class is: errors that ARE propagated but leave state inconsistent. In Rust, the `?` operator makes early returns easy but cleanup is manual — there's no `finally` block or automatic rollback.

The technique: for each `?` in a function that mutates state, check if the mutations before it are undone on the error path. This is a specific trace, not a vague review.

**Sources:**
- [Swallowed Exceptions (Harness)](https://www.harness.io/blog/swallowed-exceptions-java-applications)
- [Google Error Prone — DeadException](https://errorprone.info/bugpatterns)

### SKILL.md

```yaml
---
name: error-paths
description: Check if early returns via ? leave state inconsistent after partial mutations
disable-model-invocation: true
argument-hint: [file-path or function-name]
allowed-tools: Read Grep Glob
---

Audit error path consistency in $ARGUMENTS.

## Step 1: Find state-mutating fallible functions

Identify functions that:
1. Return `Result<_, _>` or `anyhow::Result<_>`
2. Contain state mutations (database writes, field assignments on `&mut self`, file operations, channel sends)
3. Contain at least one `?` operator AFTER a mutation

These are the candidates — a `?` after a mutation means the error path skips subsequent mutations.

## Step 2: Trace each error path

For each function, list:
- **Mutations**: ordered list of state changes (M1, M2, M3...)
- **Failure points**: each `?` and which mutations precede it

Example:
```
M1: db.insert(record)        // mutation
M2: cache.invalidate(key)?   // <-- failure here means M1 happened but M3 won't
M3: db.update_counter()
```

## Step 3: Classify each error path

For each failure point, classify the resulting state:

- **CONSISTENT**: All preceding mutations are either (a) rolled back by a guard/cleanup, (b) idempotent and safe to leave, or (c) inside a transaction that rolls back atomically
- **INCONSISTENT**: Some mutations persist while others don't, leaving the system in a state that no successful execution would produce
- **SWALLOWED**: Error is caught with `let _ =`, `.ok()`, `.unwrap_or_default()` without a comment explaining why — the error might be important

## Step 4: Check for silent discards

Search for these patterns:
- `let _ = fallible_call()`
- `.ok()` on a Result that isn't used
- `.unwrap_or_default()` where the default masks a failure
- `if let Ok(v) = ...` with no `else` branch (the Err is silently ignored)

For each, check: is the error intentionally discarded (with a comment), or is it an oversight?

## Output

### Inconsistent Error Paths
| Function | Failure Point | Persisted Mutations | Skipped Mutations | Impact |
|---|---|---|---|---|

### Silently Discarded Errors
| Location | Pattern | Discarded Error Type | Intentional? |
|---|---|---|---|

Only report cases where inconsistency is possible. Functions that use database transactions or RAII cleanup guards are safe — note them briefly but don't flag them.
```

---

## 9. Git Archaeology

**What it does:** Before you change something, reconstructs WHY the code exists in its current form by combining `git blame`, `git log --follow`, PR discussions, and constraint discovery.

**Why it's not just a prompt:** This is from dgriffith/bad-daves-robot-army and it encodes a specific investigation procedure: timeline reconstruction → PR discussion analysis → constraint discovery → pattern context. The key insight: understanding WHY code was written a certain way prevents you from "fixing" intentional design decisions. It's the difference between `git blame` (who) and archaeological reconstruction (why, under what constraints, what alternatives were rejected).

**Sources:**
- [bad-daves-robot-army](https://github.com/dgriffith/bad-daves-robot-army)
- [Claude Code Git Workflows (Beam)](https://getbeam.dev/blog/claude-code-git-workflows.html)

### SKILL.md

```yaml
---
name: archaeology
description: Reconstruct WHY code exists before changing it — git blame + PR context + constraint discovery
disable-model-invocation: true
argument-hint: [file:line-range or function-name]
allowed-tools: Bash Read Grep Glob
---

Investigate the history and rationale of $ARGUMENTS before making changes.

## Step 1: Timeline reconstruction

```bash
# Whitespace-ignoring blame with copy detection
git blame -w -C -C -C <file> -- L<start>,<end>

# Full history including renames
git log --follow --stat -p -- <file>

# Find the commit that INTRODUCED the code (not just last touched)
git log --diff-filter=A -- <file>
```

Build a timeline of significant changes to this code, noting:
- When it was introduced and by whom
- Each significant modification with its commit message
- Any reverts or reintroductions

## Step 2: PR and issue context

For each significant commit, look for PR or issue references:

```bash
# Find PRs that included this commit
git log --oneline --grep="<commit-hash-prefix>"

# Check commit message for issue/PR references (#123, JIRA-456)
git show --format="%B" <commit>
```

If GitHub PRs are available, check the discussion for:
- Rejected alternatives
- Constraints that shaped the design
- Known limitations acknowledged at the time

## Step 3: Constraint discovery

Identify forces that shaped this code:
- **Performance constraints**: Was this optimized for a specific bottleneck?
- **Compatibility constraints**: Does it handle a legacy format, API version, or edge case?
- **Safety constraints**: Is the apparent complexity protecting against a specific failure mode?
- **External constraints**: Does it work around a library bug, OS limitation, or infrastructure quirk?

Look for comments, commit messages, and nearby TODO/FIXME/HACK markers.

## Step 4: Change coupling

```bash
# What other files changed alongside this code?
git log --format="%H" -- <file> | head -20 | while read h; do
  git diff-tree --no-commit-id --name-only -r "$h"
done | sort | uniq -c | sort -rn | head -10
```

These co-changed files are likely coupled. Changing this code may require changing them too.

## Output

### Timeline
| Date | Author | What Changed | Why (from commit/PR) |
|---|---|---|---|

### Constraints Discovered
- [list each constraint with evidence]

### Coupled Files
- [files that historically change with this code]

### Recommendation
Based on the archaeology: is the proposed change safe? What constraints must be preserved? What coupled files might need updating?
```

---

## 10. Dead Reference Scanner

**What it does:** Fast, focused scan of documentation files for references to code elements that no longer exist — deleted functions, moved files, renamed types, obsolete CLI commands.

**Why it's not just a prompt:** Tan-Wagner-Treude (ICSE 2024) found that 28.9% of top GitHub projects contain outdated code references. Their technique is purely mechanical: extract identifiers from docs, check against the source tree, report mismatches. Zero false positives for deleted files — the file either exists or it doesn't. This is the highest signal-to-noise ratio of any doc-checking technique.

Unlike the broader `/doc-drift` skill, this one is fast and deterministic — no judgment required. Run it in CI.

**Sources:**
- [Detecting Outdated Code Element References (ICSE 2024)](https://link.springer.com/article/10.1007/s10664-023-10397-6)
- [markdown-link-check](https://github.com/tcort/markdown-link-check)

### SKILL.md

```yaml
---
name: dead-refs
description: Find documentation references to deleted files, renamed functions, and obsolete commands
disable-model-invocation: true
argument-hint: [doc-file or blank for all .md]
allowed-tools: Bash Read Grep Glob
---

Scan $ARGUMENTS (default: all .md files) for references to code that no longer exists.

## Step 1: Extract all code references from docs

Parse each markdown file for:

**File paths** — regex: paths containing `/` with code-like extensions, or starting with `src/`, `frontend/`, `config/`, etc.

**Backticked symbols** — content inside single backticks that matches:
- `snake_case` (function/variable names)
- `CamelCase` (type/struct names)
- `UPPER_SNAKE` (constants)
- `module::path::style` (Rust paths)
- `Type::Variant` (enum variants)

**Shell commands** — content in ```bash/sh blocks, extract the first word of each command and any path arguments.

## Step 2: Verify each reference

**File paths**: Does the file exist? `test -f <path>`
- If not, check `git log --all --follow -- <path>` for history
- If history exists: DELETED or MOVED (check for similar filename elsewhere)

**Code symbols**: Search with grep across the codebase
- If no match in current code but found in git history: DELETED
- If similar symbol exists (edit distance ≤ 2): RENAMED

**Commands**: Verify module paths in cargo test commands, binary names, etc.

## Step 3: Report

### Dead References
| Doc File | Line | Reference | Type | Status | Last Seen |
|---|---|---|---|---|---|

Type: FILE_PATH, SYMBOL, COMMAND
Status: DELETED, RENAMED (suggest new name), MOVED (suggest new path), NEVER_EXISTED

Sort by doc file, then line number. Only report confirmed dead references — if uncertain, skip it.

This skill is intentionally narrow and fast. For deeper analysis (staleness scoring, semantic drift), use /doc-drift instead.
```

---

## Appendix: Research Sources

### Academic Papers
- BugStone: One Bug, Hundreds Behind (arXiv 2510.14036) — clone divergence
- KNighter: Transforming Static Analysis with LLM-Synthesized Checkers (SOSP 2025)
- SMBugFinder: State Machine Bug Finder (ISSTA 2024)
- ConSynergy: Concurrency Bug Detection (Future Internet, 2025)
- FuzzSight: Enhancing Code Review through Fuzzing and Likely Invariants (arXiv 2510.15512)
- AutoSpec: LLM-Driven Specification Synthesis (CAV 2024)
- ROSE: Mining Version Histories (IEEE TSE 2005)
- TypePulse: Detecting Type Confusion in Rust (USENIX Security 2025)
- CARL-CCI: Comment Inconsistency Detection (ICSE 2025)
- Tan-Wagner-Treude: Outdated Code Element References (ICSE 2024)
- LLift: Enhancing Static Analysis with LLMs (OOPSLA 2024)
- DiffSpec: Differential Testing with LLMs (arXiv 2410.04249)

### Industry
- Google Bugspots Algorithm (igrigorik/bugspots)
- Adam Tornhill: Your Code as a Crime Scene
- Google Rust Crate Auditing Standards
- IBM US Patent 8,607,193 (staleness scoring)
- Cloudflare outage from unwrap()
- Mars Climate Orbiter ($125M unit confusion)
- Meta: Semi-formal Reasoning for Code Review (SOSP 2025)

### Tools & Repos
- [bad-daves-robot-army](https://github.com/dgriffith/bad-daves-robot-army) — git archaeology, blast radius
- [Claude-Command-Suite](https://github.com/qdhenry/Claude-Command-Suite) — clean arch audit, boundary detection
- [actionbook/rust-skills](https://github.com/actionbook/rust-skills) — cognitive routing for Rust
- [davidbarsky gist](https://gist.github.com/davidbarsky/8fae6dc45c294297db582378284bd1f2) — Anthropic engineer's Rust skills
- [spec-gen](https://github.com/clay-good/spec-gen) — specification drift detection
- [lockbud](https://github.com/BurtonQin/lockbud) — Rust lock analysis
- [deepdive](https://github.com/wanpengxie/deepdive) — non-linear thinking
- [awesome-claude-code](https://github.com/hesreallyhim/awesome-claude-code) — master index

### Blog Posts
- [Common Mistakes with Rust Async (Qovery)](https://www.qovery.com/blog/common-mistakes-with-rust-async)
- [How to Deadlock Tokio (Turso)](https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex)
- [Auto-Reviewing Claude's Code (O'Reilly)](https://www.oreilly.com/radar/auto-reviewing-claudes-code/)
- [Prompting for Security Reviews (CrashOverride)](https://crashoverride.com/blog/prompting-llm-security-reviews)
- [Tips for Faster Rust Compile Times (corrode)](https://corrode.dev/blog/tips-for-faster-rust-compile-times/)
