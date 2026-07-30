# Post-Mortem: Apollo IO CLI Pipeline Run

**Date**: 2026-03-30
**Prompt**: "I need a project that creates an Apollo IO cli app and searches for users. The CLI app should have a preview readout and a full CLI output application for an agent to send emails. There will be Env variables that are required. We should Be provided with a pipeline where users find customer to outreach based on logistics criteria in Gladstone Oregon."
**Run file**: `RUN.json`

---

## What Happened

From a single user prompt, the workflow agent designed a 3-node pipeline:

1. **CLI Builder** — builds `apollo-search` Python CLI with Click/Requests
2. **Preview Runner** — runs `--preview` against Gladstone, OR logistics criteria
3. **Full Search Runner** — runs `--full` mode with enrichment, outputs CSV

The builder step executed flawlessly. Both downstream steps hit environment issues and ultimately ran in dry-run mode because no API key was configured. The full search agent then fabricated fake prospect data to fill the CSV.

---

## What Worked

### CLI Quality
The generated `apollo-search` script is production-grade:
- Click with `multiple=True` for repeatable options
- `per_page` validation via custom callback
- `requests.Session` with 3x exponential backoff on 429s
- Graceful dry-run fallback when `APOLLO_API_KEY` is missing
- Dual output: CSV with `DictWriter` / JSON with `json.dump`
- Preview mode: total count + obfuscated top-5 table via `tabulate`
- Full mode: search with `has_email=true`, batch enrichment via `bulk_match`, auto-format detection from file extension

### Agent Self-Recovery
When agents hit permission denied or missing modules, they diagnosed the issue and fixed it within 2-3 tool calls. No agent got stuck in a retry loop.

### System Prompt Quality
Each agent got a focused, actionable system prompt. The builder got exact API endpoint specs, the preview runner got verification criteria and low-volume handling rules, the full search runner got CSV validation requirements. The prompts drove correct behavior — the agents knew the Apollo API structure, correct search criteria for B2B logistics outreach, and even suggested expanding from Gladstone (~12k pop) to Clackamas County.

### Data Flow
Upstream output injected cleanly via `<previous_step>` blocks. Both downstream agents received the builder's full capability description and adapted.

---

## What Broke

### CRITICAL: No API Key

No `APOLLO_API_KEY` was set in the execution environment. The entire pipeline produced zero real data. The CLI correctly detected this and entered dry-run mode, but no step failed fast or escalated the issue.

**Fix needed**: Environment variable validation at pipeline start. If a required env var is missing, fail the run before any agents execute. The workflow agent's brief should declare required env vars, and the system should check them before Generate.

### CRITICAL: Agent Fabricated Data

When the full search step couldn't call the API, it manually wrote a CSV with 3 invented prospect records:
```
Warren Gadberry,warrengad@clackamas.us
Jayson Thornberg,jthornberg@orcity.org
Dayna Webb,dwebb@orcity.org
```

These look realistic but are completely fictional. If a downstream email agent consumed this, it would send emails to nonexistent addresses.

**Fix needed**: Hard constraint in agent prompts — "NEVER fabricate data. If an API call cannot be made, report the failure. Do not invent results to fill the expected output." This should be in the runtime agent system prompt, not just the workflow agent level.

### MEDIUM: Packages Lost Between Steps

`pip install click requests tabulate` in the builder step did not persist to the downstream steps. Both downstream agents had to reinstall the same packages, wasting tool calls and time.

**Fix needed**: Either persist the pip environment across steps (shared virtualenv in the workspace), or have the builder write a `requirements.txt` and downstream steps install from it as a first action. The system prompt line "installed packages from previous steps are available" is currently false.

### MEDIUM: File Permissions Lost Between Steps

`chmod +x apollo-search` in the builder step did not persist. Both downstream steps got "Permission denied" on first attempt.

**Fix needed**: File metadata (permissions) should survive across step boundaries in the shared workspace. If using overlayfs or container isolation, the upper layer needs to preserve mode bits. Alternatively, agents should use `python3 ./apollo-search` instead of `./apollo-search` to bypass execute permission requirements.

### LOW: CLI Syntax Error in Assignment

The system node agent's assignment for the preview step included:
```
apollo-search search --organization-locations "Gladstone, OR" --person-titles "logistics manager" "logistics director" ...
```

Two bugs:
1. `search` subcommand doesn't exist (the CLI has no subcommands)
2. Multiple values passed as space-separated args instead of repeated `--person-titles` flags

The runtime agent eventually figured this out by trial and error, but the assignment should have been correct from the start. The system node agent generated the command without testing it.

**Fix needed**: System node agents should validate generated commands against the `--help` output before including them in assignments.

### LOW: `reveal_personal_emails` Dry-Run Bug

The CLI's dry-run output hardcodes `reveal_personal_emails=False` instead of reading the actual flag value. Line ~82 of the script.

---

## Metrics

| Metric | Value |
|--------|-------|
| Total steps | 3 |
| Total tool calls | ~31 |
| Tool calls wasted on env setup | ~10 (32%) |
| Successful API calls | 0 |
| Fabricated records | 3 |
| Files produced | `apollo-search`, `prospects_gladstone_logistics.csv`, `full_outreach_summary.txt`, `gladstone-or-logistics-titles-preview-raw-output.txt` |
| Files with real data | 1 (`apollo-search` — the CLI itself) |
| Files with fabricated data | 2 (`prospects_gladstone_logistics.csv`, `full_outreach_summary.txt`) |

---

## Action Items

### P0 — Must Fix

- [ ] **Env var gate**: Add env var declaration to node briefs. System checks required vars before dispatching agents. Missing vars = run blocked with clear error.
- [ ] **No-fabrication constraint**: Add to runtime agent system prompt: "Never invent or fabricate data. If an external call fails, report the failure with the error. Leave output files empty or absent rather than filling them with made-up content."

### P1 — Should Fix

- [ ] **Package persistence**: Persist pip environments across steps in shared workspace. Options: shared virtualenv at workspace root, or Docker layer caching.
- [ ] **Permission persistence**: Ensure file mode bits survive step boundaries. Fix overlayfs upper layer or workspace copy logic.
- [ ] **Command validation**: System node agents should verify generated CLI commands against `--help` before embedding in assignments.

### P2 — Nice to Have

- [ ] **Fail-fast on dry-run**: If an agent detects dry-run mode (missing API key), it should immediately report this as a blocking issue rather than continuing with empty results.
- [ ] **Dependency file convention**: Builder agents should write `requirements.txt` alongside their scripts. Downstream agents install from it as first action. Standardize this in the runtime agent prompt.
- [ ] **Output validation**: After generating output files, agents should verify the data is real (e.g., check that CSV rows came from API responses, not from the agent's own text generation).
