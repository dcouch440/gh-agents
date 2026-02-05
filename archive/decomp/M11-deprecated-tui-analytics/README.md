# Milestone 11: Usage Analytics

> Full visibility into agent activity, costs, and performance

## Goal

Provide dashboards and reports for understanding agent usage, costs, and performance over time. Surface the data that's already being collected in `cost_records` and `task_events`.

## Checkpoint

Can view `/stats` dashboard with key metrics, see detailed cost breakdown in `/costs`, track session history, and export reports to CSV/JSON.

## Dependencies

- **M1: Foundation** - Database with cost_records, task_events
- **M2: LLM Layer** - Cost tracking
- **M6: TUI Basic** - View infrastructure

## Tickets

| Ticket | Title | Slices | Status |
|--------|-------|--------|--------|
| 11.1 | Analytics Query Layer | 3 | pending |
| 11.2 | Stats Dashboard (/stats) | 4 | pending |
| 11.3 | Cost Breakdown (/costs) | 3 | pending |
| 11.4 | Session Tracking | 3 | pending |
| 11.5 | Export & Reports | 3 | pending |

**Total Slices:** 16

## Key Features

### Stats Dashboard (`/stats`)

```
┌─────────────────────────────────────────────────────────┐
│ Usage Statistics                          Last 7 days  │
├─────────────────────────────────────────────────────────┤
│  Tasks              Costs            Tokens            │
│  ──────────         ──────────       ──────────        │
│  Completed: 47      Total: $12.34    Input: 1.2M       │
│  Failed: 3          Orch: $8.20      Output: 340K      │
│  Success: 94%       Worker: $3.89    Avg/Task: 28K     │
└─────────────────────────────────────────────────────────┘
```

### Cost Breakdown (`/costs`)

- By tier (orchestrator, worker, utility)
- By model (claude-opus, claude-sonnet, etc.)
- By task
- Time filtering (today, week, month)

### Export

- CSV export for spreadsheet analysis
- JSON export for programmatic access
- `/export costs --format csv --period week`

## Technical Notes

- Most data already exists in `cost_records` and `task_events`
- Queries should be efficient with proper indexes
- Consider caching aggregates for performance
- ASCII charts keep TUI aesthetic consistent

## Parallelization

- 11.1 (Queries) must come first
- 11.2 (Stats) and 11.3 (Costs) can be parallel after 11.1
- 11.4 (Sessions) is independent
- 11.5 (Export) needs 11.1
