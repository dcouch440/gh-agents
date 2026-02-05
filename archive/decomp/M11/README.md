# Milestone 11: React Foundation

> React app scaffold with auth, routing, and layout.

## Goal

A working React application with authentication, navigation, and the Claude Code-inspired design system.

**Checkpoint**: Can login, see layout with sidebar, navigate between views.

---

## Context

This milestone creates the React frontend that connects to the Rust backend (M10).

**Architecture**:
```
┌─────────────────────────────────────────────┐
│           React App (ui/)                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐      │
│  │  Pages  │ │  Hooks  │ │  Store  │      │
│  └─────────┘ └─────────┘ └─────────┘      │
│                    │                        │
│           ┌───────┴───────┐                │
│           │   API Client  │                │
│           └───────────────┘                │
└─────────────────────────────────────────────┘
                    │
                    ▼ HTTP + WebSocket
┌─────────────────────────────────────────────┐
│            Rust Server (M10)                │
└─────────────────────────────────────────────┘
```

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 11.1 | Project Setup | 5 | M10.1 |
| 11.2 | API Client | 4 | M10.2-10.5 |
| 11.3 | Authentication UI | 4 | 11.1, 11.2, M10.5 |
| 11.4 | Layout Components | 4 | 11.1, 11.3 |

---

## Tech Stack

- **Build**: Vite
- **Framework**: React 18 + TypeScript
- **Routing**: React Router v6
- **State**: Zustand
- **Styling**: TailwindCSS
- **Icons**: Lucide React

---

## File Structure

```
ui/
├── src/
│   ├── main.tsx              # Entry point
│   ├── App.tsx               # Root component with routing
│   ├── api/
│   │   ├── client.ts         # HTTP client
│   │   └── websocket.ts      # WebSocket client
│   ├── components/
│   │   ├── Layout.tsx        # App shell
│   │   ├── Sidebar.tsx       # Navigation
│   │   ├── Header.tsx        # Top bar
│   │   └── StatusDot.tsx     # Status indicators
│   ├── pages/
│   │   ├── LoginPage.tsx
│   │   ├── SetupPage.tsx
│   │   └── DashboardPage.tsx
│   ├── hooks/
│   │   ├── useAuth.ts
│   │   └── useWebSocket.ts
│   ├── store/
│   │   └── index.ts          # Zustand store
│   └── styles/
│       └── globals.css       # Tailwind imports
├── index.html
├── package.json
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── .env.example
```

---

## Design Reference

See `doc/DESIGN-SYSTEM.md` for the Claude Code-inspired design:
- Dark background (#0d1117)
- Monospace for agent output
- Orange accent (#da7756)
- Minimal chrome

---

## Completion Criteria

- [ ] Vite dev server runs
- [ ] TailwindCSS configured with design tokens
- [ ] API client connects to Rust backend
- [ ] WebSocket connects and receives updates
- [ ] Login flow works
- [ ] Layout renders with sidebar
- [ ] Navigation between routes works
- [ ] Responsive on mobile
