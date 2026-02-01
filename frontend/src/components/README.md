# Components

## Phase 1: Primitives

Building blocks used across every page.

| Component | Purpose |
|-----------|---------|
| `StatusBadge` | Colored pill for any status string (task, agent, pipeline run) |
| `LoadingSpinner` | Consistent loading indicator |
| `ErrorMessage` | Error display with optional retry callback |
| `EmptyState` | "No items" placeholder with icon and message |
| `PageHeader` | Page title + optional action buttons |
| `Card` | Generic container with optional title |
| `DataTable` | Sortable, typed-column table for list views |
| `KeyValue` | Label/value pair row for detail pages |

## Phase 2: Domain Composites

Built from Phase 1 primitives, scoped to a specific domain.

| Component | Purpose |
|-----------|---------|
| `FeedItem` | Activity feed entry (icon by type, timestamp, message) |
| `AgentCard` | Agent summary (name, tier badge, status, current task) |
| `TaskRow` | Task in a list (title, status, priority, assigned agent) |
| `PipelineStageBar` | Visual stage progress for pipeline runs |
| `StatCard` | Single metric (label, value, trend) for dashboard |
| `ChatBubble` | Chat message (role, content, timestamp) |

## Phase 3: Complex / Interactive

Deferred until pages need them. Require more logic and/or real-time integration.

| Component | Why deferred |
|-----------|-------------|
| `ChatInput` | WS integration, command parsing |
| `PipelineGraph` | Visual DAG layout |
| `DocumentViewer` | Markdown rendering, search highlighting |
| `SettingsForm` | Form validation, config mutation |
| `GateApprovalDialog` | Modal with approval/reject actions |

## Conventions

- **Stateless and pure.** Every component is presentational only — props in, JSX out. No hooks, no context consumption, no side effects. The parent/page is responsible for data fetching and state management.
- One component per file, file name matches component name
- `function` declarations for components
- Props type colocated in the component file
- Named exports only
- Components live in `src/components/<phase-or-domain>/`
