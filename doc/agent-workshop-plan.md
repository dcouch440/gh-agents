# Agent Workshop — Full Build Plan

## Vision

A split-pane "Agent Workshop" page for designing AI agents. Chat on the left to talk to an AI about the agent's system prompt; CodeMirror editor on the right to edit/preview the prompt as markdown. Terminal aesthetic — looks like a program, not a bootstrapped website.

```
┌─────────────────────────────────────────────────────────┐
│  ◄ Agents    Agent Workshop                    [Save]   │
├────────────────────┬──┬─────────────────────────────────┤
│                    │▌▌│  [Edit] [Preview]               │
│  > user message    │▌▌│  ┌─────────────────────────┐   │
│                    │▌▌│  │ # Agent Name            │   │
│  assistant reply   │▌▌│  │                         │   │
│  with **markdown** │▌▌│  │ You are a code review   │   │
│                    │▌▌│  │ assistant that...        │   │
│                    │▌▌│  │                         │   │
│                    │▌▌│  └─────────────────────────┘   │
│                    │▌▌│                                 │
│                    │▌▌│  Model: [Sonnet ▾]              │
│                    │▌▌│  Max Tokens: [4096]             │
│                    │▌▌│  Temperature: [0.7]             │
├────────────────────┤▌▌├─────────────────────────────────┤
│  > _               │▌▌│                                 │
└────────────────────┴──┴─────────────────────────────────┘
        chat panel    ↑         editor panel
                   drag handle
```

---

# Part 1: Reusable Primitives

Primitives only. No page, no routing, no API wiring. Pure stateless components — props in, JSX out. Tests in isolation.

## Install

```bash
npm install react-markdown remark-gfm remark-breaks
```

## Components

### SplitPane — `src/components/primitives/SplitPane.tsx`

```typescript
type SplitPaneProps = {
  left: ReactNode
  right: ReactNode
  initialSplit?: number  // 0-100, default 50
  minLeft?: number       // default 20
  maxLeft?: number       // default 80
  className?: string
}
```

Stateless rendering, parent page passes children. Drag state lives in a `useSplitPane` hook (separate file) that returns `{ splitPercent, handleMouseDown }`.

---

### useSplitPane — `src/hooks/useSplitPane.ts`

```typescript
const useSplitPane = (opts: { initial: number; min: number; max: number }) =>
  { splitPercent: number; handleMouseDown: (e: React.MouseEvent) => void }
```

Owns the drag logic. `mousedown` on handle → `mousemove`/`mouseup` on document. Clamps to min/max. Adds `user-select: none` to body during drag.

---

### ChatMessage — `src/components/chat/ChatMessage.tsx`

```typescript
type ChatMessageProps = {
  role: 'user' | 'assistant'
  content: string
  streaming?: boolean
}
```

Terminal aesthetic (ported from archive/ui Message component):
- **User**: `--color-text-muted`, `0.8125rem`, `pre-wrap`, monospace
- **Assistant**: `--color-text`, `0.875rem`, rendered via MarkdownPreview, `word-wrap: break-word`
- **Streaming cursor**: `2px × 1.1em` inline-block, `--color-accent` bg, `1s cubic-bezier(0.4,0,0.6,1)` blink animation
- **Hover actions**: copy button fades in (`opacity 0→1, 150ms`), positioned below message
- No bubbles, no backgrounds — flat left-aligned text on dark bg

---

### ChatInput — `src/components/chat/ChatInput.tsx`

```typescript
type ChatInputProps = {
  onSend: (message: string) => void
  disabled?: boolean
  placeholder?: string
}
```

Owns its own `value` state (the only state — it's an input). Auto-expanding textarea (max 6 rows). Enter sends + clears, Shift+Enter newline. `border-top` separator only. No send button.

---

### ChatPanel — `src/components/chat/ChatPanel.tsx`

```typescript
type ChatMessageData = {
  id: string
  role: 'user' | 'assistant'
  content: string
}

type ChatPanelProps = {
  messages: ChatMessageData[]
  onSend: (message: string) => void
  streaming?: boolean
  disabled?: boolean
  className?: string
}
```

Composes ChatMessage list + ChatInput. Auto-scroll to bottom (only if already at bottom). Last assistant message gets `streaming` prop. Empty state when no messages.

---

### MarkdownPreview — `src/components/primitives/MarkdownPreview.tsx`

```typescript
type MarkdownPreviewProps = {
  content: string
  className?: string
}
```

`react-markdown` + `remark-gfm` + `remark-breaks`. Strips `<thinking>` tags. Custom component overrides:
- `code` inline: `--color-accent`, monospace, `0.875em`
- `code` block: `border-left: 2px solid --color-border`, padding `0.5rem 0.75rem`, monospace `0.8125rem`, `white-space: pre`
- `pre`: passthrough (code block handles rendering)
- `p`: `margin-bottom: 0.25rem`, last-child 0
- `ul`/`ol`: disc/decimal, inside position, `0.25rem` bottom margin
- `table`: full-width collapse, `th` with semibold secondary color + border-bottom, `td` with muted border
- Scrollable container, full height

---

### EditorToolbar — `src/components/primitives/EditorToolbar.tsx`

```typescript
type EditorToolbarProps = {
  children: ReactNode
  className?: string
}
```

Pure. Flex row container with gap. Thin padding. Bottom border.

---

### ToggleGroup — `src/components/primitives/ToggleGroup.tsx`

```typescript
type ToggleOption = {
  value: string
  label: string
}

type ToggleGroupProps = {
  options: ToggleOption[]
  value: string
  onChange: (value: string) => void
  className?: string
}
```

Pure. Row of toggle buttons, one active at a time. For the Edit/Preview switch and anywhere else.

---

## Files

| File | Type |
|------|------|
| `src/components/primitives/SplitPane.tsx` + test | Stateless layout |
| `src/hooks/useSplitPane.ts` + test | Stateful hook |
| `src/components/chat/ChatMessage.tsx` + test | Stateless |
| `src/components/chat/ChatInput.tsx` + test | Stateful (input value only) |
| `src/components/chat/ChatPanel.tsx` + test | Stateless (composes above) |
| `src/components/chat/index.ts` | Barrel export |
| `src/components/primitives/MarkdownPreview.tsx` + test | Stateless |
| `src/components/primitives/EditorToolbar.tsx` + test | Stateless |
| `src/components/primitives/ToggleGroup.tsx` + test | Stateless |
| `src/components/primitives/index.ts` | Add exports |
| `src/styles/components.css` | All new CSS |

## CSS Classes

```
/* SplitPane */
.split-pane, .split-pane__left, .split-pane__right, .split-pane__handle

/* Chat */
.chat-panel, .chat-panel__messages, .chat-panel__empty
.chat-message, .chat-message--user, .chat-message--assistant, .chat-message__cursor
.chat-input, .chat-input__textarea

/* Markdown Preview */
.markdown-preview (+ h1-h3, p, ul, ol, code, pre, blockquote styles)

/* Editor Toolbar */
.editor-toolbar

/* Toggle Group */
.toggle-group, .toggle-group__btn, .toggle-group__btn--active
```

## Verify

```bash
cd frontend && npx tsc --noEmit && npx eslint . && npx vitest run
```

---

# Part 2: Agent Workshop Page

Wire primitives into the actual page. Update types. Add routing.

## Update Agent Types — `src/types/agent.ts`

Align with DB schema (`doc/database-model-guide.md`):

```typescript
type Agent = {
  id: string
  name: string              // was persona_name
  system_prompt: string     // was persona_prompt
  model_provider: string
  model_id: string
  model_max_tokens: number
  model_temperature: number
  created_at: string
  updated_at: string
}

type CreateAgentRequest = {
  name: string
  system_prompt?: string
  model_provider?: string
  model_id?: string
  model_max_tokens?: number
  model_temperature?: number
}
```

Remove `AgentTier`, `AgentStatus`, `persona_name`, `persona_prompt`, `persona_style`, `tier`, `status`. Update all references.

## AgentWorkshopPage — `src/pages/Agents/AgentWorkshopPage.tsx`

```typescript
type WorkshopState = {
  name: string
  systemPrompt: string
  modelId: string
  maxTokens: number
  temperature: number
  editorMode: 'edit' | 'preview'
  messages: ChatMessageData[]
  streaming: boolean
}
```

- SplitPane: chat left, editor right
- Left: ChatPanel wired to `useSendSessionMessage` for SSE streaming
- Right: EditorToolbar with ToggleGroup (Edit/Preview), CodeEditor or MarkdownPreview, model config fields
- PageHeader with back link + Save button

## Routing

- `/agents/workshop` (new) and `/agents/workshop/:id` (edit existing)
- Add to constants, router, RouteWrappers
- AgentsPage "Create Agent" → workshop

## Files

| File | Action |
|------|--------|
| `src/types/agent.ts` | Rewrite to match DB |
| `src/pages/Agents/AgentWorkshopPage.tsx` + test | New page |
| `src/pages/Agents/CreateAgentPage.tsx` + test | Delete |
| `src/constants.ts` | Add AGENT_WORKSHOP route |
| `src/router.tsx` | Add workshop route |
| `src/RouteWrappers.tsx` | Add workshop wrapper |
| `src/test/fixtures.ts` | Update mock agent shape |
| All files referencing old Agent type | Update field names |

---

# Part 3: Backend Agent Integration

## Workshop Agent Session

- Create/resume chat session via `api.sessions.create()`
- Messages through `useSendSessionMessage` → SSE stream from backend
- Backend agent receives current system prompt draft as context

## Document Editing Tools (Future)

Agent tools to directly modify the system prompt:
- `replace_section(heading, content)` — find markdown heading, replace content
- `append_section(heading, content)` — add new section
- `set_field(field, value)` — update model config

Implemented via Rust backend tool router (`doc/tool-router-design.md`).

## Save Flow

- Save button → `api.agents.create(body)` or `api.agents.update(id, body)`
- Navigate to agent detail on success
- Dirty state tracking (warn on unsaved navigation)

---

# Design Decisions

1. **Terminal aesthetic** — monospace, muted colors, no rounded corners/shadows
2. **Stateless components** — pure functions. State in hooks or page reducers
3. **DB-aligned types** — frontend types match PostgreSQL schema exactly
4. **SplitPane as primitive** — reusable for any two-panel layout
5. **MarkdownPreview shared** — used in ChatMessage and editor preview mode
6. **CodeEditor exists** — CodeMirror 6 wrapper already built, just needs wiring
