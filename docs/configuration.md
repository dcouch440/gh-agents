# Configuration

nexor uses layered configuration: global defaults -> project overrides -> environment variables.

## Config File Locations

| Location | Purpose |
|----------|---------|
| `~/.config/nexor/config.toml` | Global defaults |
| `.nexor/config.toml` | Project-specific overrides |

Project config overrides global config. Environment variables override both.

## Full Configuration Reference

```toml
# ~/.config/nexor/config.toml

# =============================================================================
# Model Configuration
# =============================================================================
[models]
# Orchestrator: planning, review, decisions (expensive tier)
orchestrator = { provider = "anthropic", model = "claude-sonnet-4-20250514", max_tokens = 8192 }

# Worker: code implementation (mid tier)
worker = { provider = "anthropic", model = "claude-sonnet-4-20250514", max_tokens = 4096 }

# Utility: formatting, linting, docs (cheap tier)
utility = { provider = "anthropic", model = "claude-haiku", max_tokens = 2048 }

# =============================================================================
# Agent Pool
# =============================================================================
[pool]
max_orchestrators = 2
max_workers = 6
max_utilities = 4

# =============================================================================
# UI Settings
# =============================================================================
[ui]
# Verbosity: quiet, normal, verbose
verbosity = "normal"

# =============================================================================
# Autonomy Settings (project-level recommended)
# =============================================================================
[autonomy]
# Level: full_auto, approval_gates, supervised
level = "approval_gates"

[approval_gates]
before_commit = false
before_pr = true
before_merge = true

# =============================================================================
# Git Strategy
# =============================================================================
[git]
# Strategy: branch_per_slice, branch_per_ticket
strategy = "branch_per_slice"

# =============================================================================
# Sandbox Mode
# =============================================================================
[sandbox]
# Mode: docker, local_restricted, none
mode = "docker"

# =============================================================================
# Custom Personas (optional)
# =============================================================================
[personas.orchestrator]
name = "Arch"
system_prompt = """
You are a senior software architect...
"""

[personas.worker]
name = "Dev"
system_prompt = """
You are a focused developer...
"""
```

## Common Configurations

### Conservative (Heavy Supervision)

Best for sensitive codebases or when learning to trust the system.

```toml
[autonomy]
level = "supervised"

[approval_gates]
before_commit = true
before_pr = true
before_merge = true
```

### Balanced (Default)

Good for most projects. PRs and merges need approval.

```toml
[autonomy]
level = "approval_gates"

[approval_gates]
before_commit = false
before_pr = true
before_merge = true
```

### Autonomous (Minimal Oversight)

For trusted, well-tested codebases.

```toml
[autonomy]
level = "full_auto"
```

### Cost-Conscious

Use cheaper models when possible.

```toml
[models]
orchestrator = { provider = "anthropic", model = "claude-sonnet-4-20250514", max_tokens = 4096 }
worker = { provider = "anthropic", model = "claude-haiku", max_tokens = 2048 }
utility = { provider = "anthropic", model = "claude-haiku", max_tokens = 1024 }

[pool]
max_workers = 3
```

## Environment Variables

| Variable | Overrides | Description |
|----------|-----------|-------------|
| `ANTHROPIC_API_KEY` | API auth | Required for LLM calls |
| `GITHUB_TOKEN` | GitHub auth | For GitHub integration |
| `RUST_LOG` | Log level | info, debug, trace |
| `NEXOR_CONFIG` | Config path | Override config file |

## Per-Project Configuration

Create `.nexor/config.toml` in your project root to override global settings:

```toml
# .nexor/config.toml

# Use cheaper model for this project
[models]
worker = { provider = "anthropic", model = "claude-haiku", max_tokens = 2048 }

# Require approval for all commits (sensitive project)
[approval_gates]
before_commit = true
```

## Config Validation

nexor validates config on startup. Invalid config shows an error with the issue:

```
Error: configuration error: invalid model name 'claude-unknown'
  -> Expected one of: claude-sonnet-4-20250514, claude-haiku
```

## Generating Default Config

Create a default config file:

```bash
nexor --generate-config > ~/.config/nexor/config.toml
```

Or copy the example:

```bash
cp examples/config.toml ~/.config/nexor/config.toml
```
