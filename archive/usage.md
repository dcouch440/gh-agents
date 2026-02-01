# Usage Guide

## Getting Started

### First Run

```bash
cd your-project
nexor
```

You'll see the nexor home screen:

```
+-------------------------------------------------------------+
| w[0/6] o[0/2]                                               |
|                                                             |
|            ███╗   ██╗███████╗██╗  ██╗ ██████╗ ██████╗      |
|            ████╗  ██║██╔════╝╚██╗██╔╝██╔═══██╗██╔══██╗     |
|            ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║██████╔╝     |
|            ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║██╔══██╗     |
|            ██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝██║  ██║     |
|            ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝     |
|                                                             |
|  > _                                                        |
+-------------------------------------------------------------+
```

Just start typing to chat with the orchestrator.

### Basic Workflow

1. **Describe what you want**: "Add user authentication to the API"
2. **Review the plan**: Orchestrator breaks it into slices
3. **Approve or adjust**: Confirm the approach
4. **Watch progress**: Agents work in `/feed` view
5. **Review results**: PR created when done

## Common Tasks

### Fix a Bug

```
You: There's a bug where login fails for users with @ in their email

Orchestrator: I'll investigate this. Let me look at the auth module...
[Shows investigation]

Found the issue - the email parsing regex doesn't handle @ correctly.
I'll create a fix with a test case.

[Fix applied, test added, PR created]
```

### Add a Feature

```
You: Add a /health endpoint that returns service status

Orchestrator: I'll break this into slices:
1. Create endpoint route
2. Add status checks (db, redis, external services)
3. Add tests
4. Update API documentation

Shall I proceed?

You: yes

[Agents execute each slice]
```

### Work on a GitHub Issue

```
You: /sync https://github.com/org/repo/issues/42

Orchestrator: Synced issue #42: "Add rate limiting"
Context: User wants rate limiting on API endpoints to prevent abuse.

I'll decompose this into slices:
1. Add rate limiter middleware
2. Configure limits per endpoint
3. Add 429 response handling
4. Add tests
5. Update docs

Ready to start?
```

### Review Code

```
You: Review the changes in src/auth.rs

Orchestrator: Looking at src/auth.rs...

Code Review:
- Line 45: Consider using constant-time comparison for password checks
- Line 78: This error message leaks information about valid usernames
- Line 102: Good use of rate limiting here

Suggestions:
1. Replace == with constant_time_eq for password
2. Use generic "Invalid credentials" error message

Want me to apply these suggestions?
```

### Refactor Code

```
You: /refactor

[Enters refactor mode]

Orchestrator: Refactor mode active. What would you like to improve?

You: Extract the validation logic from UserController into a separate service

Orchestrator: I'll analyze the current structure...
[Shows proposed changes]

These changes will:
- Create ValidationService class
- Move validate_email, validate_password, validate_username methods
- Update UserController to use ValidationService
- Update tests

Apply changes?
```

## Navigation

### Views

| Command | View | Purpose |
|---------|------|---------|
| `/home` | Home | Return to home screen |
| `/main` | Chat | Talk to orchestrator |
| `/feed` | Feed | Watch agent activity |
| `/logs` | Logs | Technical debug logs |
| `/tasks` | Tasks | View task queue |
| `/agents` | Agents | View agent pool |
| `/costs` | Costs | View spending breakdown |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Send message / Submit |
| `Esc` | Return to home |
| `Ctrl+C` | Exit nexor |
| `↑/↓` | Scroll in feed/logs |
| `e` | Expand errors |
| `d` | Dismiss errors |

## Headless Mode

For CI/CD or scripting:

```bash
# Single task
nexor --headless --task "run tests and fix failures"

# From file
nexor --headless --input tasks.txt --output results.txt

# GitHub issue
nexor --headless --sync "https://github.com/org/repo/issues/42"
```

## Best Practices

### Be Specific

Vague requests lead to misunderstandings:

| Instead of... | Try... |
|---------------|--------|
| "Make the app better" | "Add input validation to the registration form" |
| "Fix the bug" | "Fix the 500 error when uploading files > 10MB" |
| "Add tests" | "Add unit tests for the PaymentService class" |

### Review Plans

Always review the orchestrator's decomposition before approving. It's easier to catch misunderstandings early than to fix them later.

### Start Small

For your first task, try something simple:
- "Add a /ping endpoint that returns 'pong'"
- "Add a comment to explain what function X does"
- "Fix the typo in README.md"

### Use GitHub Issues

Syncing issues provides context the orchestrator can use:
- Issue description
- Labels (bug, feature, etc.)
- Comments and discussion
- Related PRs

### Monitor Costs

Check `/costs` periodically to understand spending patterns. Consider:
- Using cheaper models for simple tasks
- Reducing agent pool size
- Setting cost limits per task

### Trust but Verify

Review generated code before merging. Agents are helpful but not infallible:
- Check for security issues
- Verify business logic
- Run tests locally

## Troubleshooting

### Agent Stuck

If an agent seems stuck:
1. Check `/logs` for errors
2. Try `/tasks` to see queue status
3. Provide more context or clarification

### Wrong Approach

If the plan looks wrong:
1. Say "stop" or "wait"
2. Clarify what you actually want
3. Ask for an alternative approach

### Cost Concerns

If spending is too high:
1. Use `/costs` to identify expensive operations
2. Adjust model config to use cheaper models
3. Reduce agent pool size

## Next Steps

- [Command Reference](./commands.md) - All available commands
- [Configuration Guide](./configuration.md) - Customize settings
- [Docker Guide](./docker.md) - Run in container
