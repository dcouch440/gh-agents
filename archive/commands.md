# Command Reference

All commands start with `/`. Type `/help` to see this list in-app.

## Navigation Commands

### /home

Return to home screen.

```
/home
```

### /main

Chat with the orchestrator.

```
/main
```

### /feed

View real-time agent activity.

```
/feed
```

### /logs

View technical logs.

```
/logs
/logs error    # Filter by level
/logs debug
```

### /tasks

View task queue.

```
/tasks
```

### /agents

View agent pool status.

```
/agents
```

### /costs

View cost breakdown.

```
/costs
```

## Action Commands

### /sync

Sync a GitHub issue as a ticket.

```
/sync <url>
/sync https://github.com/owner/repo/issues/123
/sync owner/repo#123
```

### /refactor

Enter refactor mode for safe code changes.

```
/refactor
```

In refactor mode:
- Describe the change you want
- Review proposed changes
- Approve or reject

### /replay

View agent decision history for a task.

```
/replay <task_id>
/replay abc123
```

### /quit

Exit nexor.

```
/quit
/q
/exit
```

### /help

Show command help.

```
/help
/?
```

## CLI Arguments

When starting nexor from the command line:

```
nexor [OPTIONS]

OPTIONS:
  -H, --headless           Run without TUI
  -t, --task <TEXT>        Task to process (headless)
  -i, --input <FILE>       Read tasks from file (headless)
  -o, --output <FILE>      Write output to file (headless)
  -c, --config <FILE>      Override config file
  -v, --verbose            Increase log verbosity (-v, -vv, -vvv)
      --sync <URL>         Sync GitHub issue on start
  -h, --help               Show help
  -V, --version            Show version
```

## CLI Examples

### Interactive TUI

```bash
# Start nexor in current directory
nexor

# With verbose logging
nexor -v

# With custom config
nexor --config ./my-config.toml
```

### Headless Mode

```bash
# Single task
nexor --headless --task "add unit tests for auth module"

# Process task file
nexor --headless --input tasks.txt --output results.txt

# GitHub issue
nexor --headless --sync "https://github.com/org/repo/issues/42"
```

### Task File Format

Tasks can be plain text (one per line) or JSON:

**Plain text (tasks.txt):**
```
Add input validation
Fix login bug
Update documentation
```

**JSON format:**
```json
[
  {"description": "Add input validation", "priority": "high"},
  {"description": "Fix login bug", "priority": "critical"},
  {"description": "Update documentation"}
]
```

## Keyboard Shortcuts

| Key | Action | Context |
|-----|--------|---------|
| `Enter` | Send message / Submit | All views |
| `Esc` | Return to home | All views |
| `Ctrl+C` | Exit nexor | All views |
| `↑` | Scroll up | Feed, Logs |
| `↓` | Scroll down | Feed, Logs |
| `e` | Expand errors | When errors present |
| `d` | Dismiss errors | When errors expanded |

## Command Aliases

Some commands have shorter aliases:

| Command | Alias |
|---------|-------|
| `/quit` | `/q`, `/exit` |
| `/help` | `/?` |
| `/home` | `/h` |

## Error Codes

When commands fail, you'll see error codes:

| Code | Meaning |
|------|---------|
| `E001` | Unknown command |
| `E002` | Missing argument |
| `E003` | Invalid URL format |
| `E004` | GitHub API error |
| `E005` | Task not found |

## See Also

- [Usage Guide](./usage.md) - How to use nexor effectively
- [Configuration](./configuration.md) - All config options
- [Docker](./docker.md) - Running in containers
