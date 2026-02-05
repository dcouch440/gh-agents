# Milestone 7: Execution Layer

> Agents can read/write files, run git commands, execute tests in a safe, auditable manner.

## Goal

The execution layer provides agents with controlled access to the filesystem, git operations, test execution, and sandboxed command execution. All operations are scoped, validated, and logged for auditability and safety.

**Checkpoint**: Agent can modify a file, commit it, run tests - all with proper logging and approval gates.

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 7.1 | File Operations | 4 | M1 (Foundation - types, config) |
| 7.2 | Git Operations | 6 | M1 (Foundation) |
| 7.3 | Test Runner | 4 | M1 (Foundation) |
| 7.4 | Docker Sandbox | 4 | 7.1, 7.2, 7.3 |
| 7.5 | Approval Gates | 4 | M6 (TUI for prompts) |
| 7.6 | Git Merge Operations | 6 | 7.2 (Git Operations) |

**Total Slices**: 28

---

## Dependency Graph

```
M1 (Foundation) ──┬──→ 7.1 (File Operations) ──┐
                  │                             │
                  ├──→ 7.2 (Git Operations) ────┼──→ 7.4 (Docker Sandbox)
                  │         │                   │
                  │         └──→ 7.6 (Git Merge Operations)
                  │
                  └──→ 7.3 (Test Runner) ───────┘

M6 (TUI) ──────────────→ 7.5 (Approval Gates)
```

---

## Parallelization

**Can run in parallel**:
- 7.1, 7.2, 7.3 (all depend only on M1, no interdependencies)
- 7.5 can run in parallel with 7.1-7.3 if M6 is complete

**Must be sequential**:
- 7.4 (Docker Sandbox) needs 7.1-7.3 complete first
- 7.5 integration with 7.1-7.3 (adding approval checks) comes last
- 7.6 (Git Merge Operations) needs 7.2 complete first

**Recommended execution order**:
1. Start with 7.1, 7.2, 7.3 in parallel (all depend only on M1)
2. After 7.2: Start 7.6 (Git Merge Operations)
3. After 7.1, 7.2, 7.3: Start 7.4 (Docker Sandbox)
4. After M6 complete: Start 7.5 (Approval Gates)

---

## Key Files

All execution code lives in `src/execution/`:

```
src/execution/
├── mod.rs         ← Module exports, shared types, ExecutionContext
├── files.rs       ← File read/write with path scoping
├── git.rs         ← Git operations wrapper (status, branch, commit, diff, push)
├── git_merge.rs   ← Git merge operations (fetch, merge, conflict resolution)
├── tests.rs       ← Test runner with framework detection
├── sandbox.rs     ← Docker sandbox execution
└── approval.rs    ← Approval gate system
```

---

## External Dependencies

This milestone builds on:

- **M1 Types**: `Task`, `TaskId`, config types
- **M1 Config**: Project config for sandbox mode, approval gates
- **M1 Logging**: Tracing for audit logs
- **M1 Database**: Storing operation audit records
- **M6 TUI**: Approval prompt display (for 7.5)

---

## Security Considerations

This milestone is security-critical:

1. **Path Scoping**: All file operations must be constrained to project directory
2. **Command Injection**: Git and test commands must sanitize inputs
3. **Resource Limits**: Docker sandbox enforces CPU/memory/time limits
4. **Audit Trail**: Every operation logged for review
5. **Approval Gates**: Dangerous operations require human confirmation

---

## Notes

- File operations use async I/O via tokio::fs
- Git operations wrap the `git2` crate or shell out to git CLI
- Test runner auto-detects framework from project files
- Docker sandbox is optional - falls back to restricted local execution
- Approval gates integrate with TUI for user confirmation
