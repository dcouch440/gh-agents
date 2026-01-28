# nexor Design System

> Claude Code-inspired aesthetic for the React frontend

## Philosophy

nexor's UI should feel like a **premium developer tool** - similar to Claude Code's terminal interface, but adapted for the web. The design should:

1. **Feel familiar** to Claude Code users
2. **Prioritize content** over chrome
3. **Be functional** not decorative
4. **Work in any lighting** (dark mode primary)

---

## Color Palette

### Dark Theme (Primary)

```css
:root {
  /* Backgrounds */
  --bg-primary: #0d1117;      /* Main background */
  --bg-secondary: #161b22;    /* Cards, elevated surfaces */
  --bg-tertiary: #21262d;     /* Hover states, borders */

  /* Text */
  --text-primary: #e6edf3;    /* Main text */
  --text-secondary: #8b949e;  /* Muted text */
  --text-tertiary: #6e7681;   /* Very muted */

  /* Accent */
  --accent-primary: #da7756;  /* Claude orange/coral */
  --accent-secondary: #58a6ff; /* Links, info */

  /* Status */
  --status-success: #3fb950;
  --status-warning: #d29922;
  --status-error: #f85149;
  --status-info: #58a6ff;

  /* Borders */
  --border-default: #30363d;
  --border-muted: #21262d;
}
```

### Light Theme (Secondary)

```css
:root.light {
  --bg-primary: #ffffff;
  --bg-secondary: #f6f8fa;
  --bg-tertiary: #eaeef2;

  --text-primary: #1f2328;
  --text-secondary: #656d76;
  --text-tertiary: #8c959f;

  --accent-primary: #c4452d;
  --accent-secondary: #0969da;

  --border-default: #d0d7de;
  --border-muted: #eaeef2;
}
```

---

## Typography

### Font Stack

```css
/* UI Text */
--font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif;

/* Code / Monospace */
--font-mono: "SF Mono", "Fira Code", "Fira Mono", Menlo, Consolas, monospace;
```

### Scale

| Name | Size | Weight | Usage |
|------|------|--------|-------|
| `xs` | 12px | 400 | Timestamps, labels |
| `sm` | 14px | 400 | Secondary text, captions |
| `base` | 16px | 400 | Body text |
| `lg` | 18px | 500 | Section headers |
| `xl` | 24px | 600 | Page titles |

### Rules

- **UI text**: Sans-serif
- **Code/output**: Monospace
- **Agent messages**: Monospace (like terminal output)
- **User input**: Sans-serif

---

## Components

### Chat Message (User)

```
┌─────────────────────────────────────────────────────────────┐
│  You                                           10:42:15 AM  │
│  ───────────────────────────────────────────────────────── │
│  Add user authentication with JWT                          │
└─────────────────────────────────────────────────────────────┘
```

- Right-aligned or distinct background
- Timestamp in muted text
- Sans-serif font

### Chat Message (Agent)

```
┌─────────────────────────────────────────────────────────────┐
│  Orchestrator                                  10:42:18 AM  │
│  ───────────────────────────────────────────────────────── │
│  I'll break this into 4 vertical slices:                   │
│                                                             │
│  1. User model + database migration                         │
│  2. Register & login endpoints                              │
│  3. JWT middleware + refresh token flow                     │
│  4. Protected route examples + tests                        │
│                                                             │
│  Should I proceed with this plan?                           │
└─────────────────────────────────────────────────────────────┘
```

- Monospace font (terminal feel)
- Streaming animation (cursor blink while generating)
- Code blocks syntax highlighted

### Feed Item

```
┌─────────────────────────────────────────────────────────────┐
│  ● Worker 1                                    10:42:31     │
│    Looking at the existing user model in src/models/...     │
└─────────────────────────────────────────────────────────────┘
```

- Colored dot for agent tier (orange=orchestrator, blue=worker, gray=utility)
- Compact, log-style
- Monospace font

### Task Card

```
┌─────────────────────────────────────────────────────────────┐
│  ● In Progress                                              │
│  ──────────────────────────────────────────────────────────│
│  Add user authentication                                    │
│                                                             │
│  Slice 2 of 4 • Worker 1 • 3m elapsed                      │
└─────────────────────────────────────────────────────────────┘
```

- Status indicator with color
- Progress shown as "X of Y" not progress bar
- Elapsed time, not ETA

### Status Indicators

| State | Visual |
|-------|--------|
| Idle | ○ Gray outline |
| Working | ● Blue filled, subtle pulse |
| Success | ● Green filled |
| Warning | ● Yellow filled |
| Error | ● Red filled |

---

## Layout

### Sidebar

```
┌─────────────────┐
│  nexor          │  ← Logo/wordmark
│─────────────────│
│  ◉ Chat         │  ← Active (filled)
│  ○ Feed         │
│  ○ Tasks        │
│  ○ Agents       │
│  ○ Files        │
│  ○ Stats        │
│─────────────────│
│                 │
│                 │
│─────────────────│
│  ⚙ Settings     │
│  david@...      │  ← User/account
└─────────────────┘
```

- Width: 200-240px (collapsible to icons on mobile)
- Fixed position
- Subtle hover states

### Header Bar

```
┌─────────────────────────────────────────────────────────────┐
│  Chat                               w[2/6] o[1/1]  ●        │
└─────────────────────────────────────────────────────────────┘
```

- Page title left
- Agent counts right (like Claude Code's status)
- Connection indicator

### Main Content

- Max-width: 800-900px for readability
- Centered with padding
- Scrollable with subtle scrollbar

---

## Animations

### Streaming Text

- Characters appear one at a time (or in small chunks)
- Cursor blinks at end while generating
- Smooth scroll to keep latest text visible

### Loading States

- Subtle pulse on status indicators
- Skeleton screens for initial load (not spinners)
- No jarring transitions

### Transitions

- Page transitions: fade (100-150ms)
- Sidebar collapse: slide (150ms)
- Modal: fade + scale (150ms)

---

## Tailwind Config

```js
// tailwind.config.js
module.exports = {
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        bg: {
          primary: '#0d1117',
          secondary: '#161b22',
          tertiary: '#21262d',
        },
        text: {
          primary: '#e6edf3',
          secondary: '#8b949e',
          tertiary: '#6e7681',
        },
        accent: {
          DEFAULT: '#da7756',
          secondary: '#58a6ff',
        },
        border: {
          DEFAULT: '#30363d',
          muted: '#21262d',
        },
      },
      fontFamily: {
        sans: ['-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'sans-serif'],
        mono: ['SF Mono', 'Fira Code', 'Consolas', 'monospace'],
      },
    },
  },
}
```

---

## Reference: Claude Code Screenshots

Key visual elements to emulate:

1. **Dark background** with high contrast text
2. **Monospace output** for agent responses
3. **Colored status indicators** (dots, not badges)
4. **Minimal borders** - content defines sections
5. **Streaming text** animation
6. **Compact information density** - lots of info, little chrome

---

## Implementation Notes

### React Components to Build

1. `<Message>` - Chat message with streaming support
2. `<FeedItem>` - Compact agent activity item
3. `<TaskCard>` - Task status card
4. `<StatusDot>` - Colored status indicator
5. `<CodeBlock>` - Syntax highlighted code
6. `<Sidebar>` - Navigation sidebar
7. `<Header>` - Top bar with status

### Libraries

- **Syntax Highlighting**: Shiki or Prism (dark theme)
- **Markdown**: react-markdown with custom components
- **Icons**: Lucide React (simple, consistent)
- **Animations**: Framer Motion (subtle only)

---

*Last updated: 2026-01-27*
