# UI Design Guide

Design philosophy and patterns for the nexor UI.

## Design Direction

**Professional, terminal-inspired, understated.** Think developer tool / command center, not chatbot. Reference: Claude Code's turn-based conversation layout.

### Principles

- Monospace where it matters (code, logs, agent output)
- Sans-serif for UI chrome (headers, labels, navigation)
- Generous whitespace, tight information density where useful
- Subtle borders and dividers instead of colored backgrounds
- Accent color used sparingly (status indicators, active states only)

## Color Usage

| Purpose | Variable | When to use |
|---------|----------|-------------|
| Accent (`--color-accent`) | `#da7756` | Active states, role labels, send button. Sparingly. |
| Secondary accent (`--color-accent-secondary`) | `#58a6ff` | Worker agent labels, links |
| Status colors | `--color-status-*` | Status dots, left borders on feed items. Never as backgrounds. |
| Borders | `--color-border` / `--color-border-muted` | Dividers between sections. Muted for subtle separators (turn dividers). |

## Typography

| Context | Font | Size |
|---------|------|------|
| UI chrome (headers, nav, labels) | `--font-family-sans` | Varies |
| User messages | `--font-family-sans` | `inherit` |
| Assistant/agent output | `--font-family-mono` | `0.875rem` |
| Feed items | `--font-family-mono` | `0.8125rem` |
| Timestamps, metadata | `--font-family-sans` | `0.75rem` |

## Component Patterns

### Chat Messages (Turn Layout)

- Both user and assistant messages left-aligned
- No avatars, no bubbles, no colored backgrounds
- Thin top-border separator between turns
- Role label ("You" / "nexor") as header with timestamp
- Assistant output in monospace

### Feed Items (Log Layout)

- Monospace, tight horizontal layout
- Left-border colored by event type
- Hover state with subtle background
- Truncate long content, expand on click
- No colored backgrounds for type variants (just border color)

### Input

- Auto-growing textarea (1 to 6 lines)
- Send button appears only when there is text
- No hint text (standard Enter-to-send behavior)
- Subtle focus ring using accent color

### Empty States

- Centered, understated
- Brand name in monospace
- Tagline in smaller sans-serif
- No icons or illustrations
