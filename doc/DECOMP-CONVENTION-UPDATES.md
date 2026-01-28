# Decomp File Convention Updates

> Files that need updating to match production React conventions from CONVENTIONS.md

---

## Summary

The M11 and M12 decomp files were created before the production React conventions were established. They use quick prototype patterns instead of production-ready code.

**Total files needing updates:** 10 files
- M11: 4 files (11.1, 11.2, 11.3, 11.4)
- M12: 6 files (12.1, 12.2, 12.3, 12.4, 12.5, 12.6)

---

## What Needs Fixing

### Current (Prototype) Pattern
```tsx
// ❌ Inline Tailwind everywhere
<button className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600">
  Click me
</button>

// ❌ Basic props, no HTML attributes extension
interface ButtonProps {
  onClick: () => void;
  disabled?: boolean;
}

// ❌ No forwardRef
export function Button({ onClick, disabled }: ButtonProps) {
  return <button onClick={onClick} disabled={disabled}>Click</button>;
}
```

### Production Pattern (from CONVENTIONS.md)
```tsx
// ✅ CSS Modules for styling
import styles from './Button.module.css';

// ✅ Extends HTML attributes
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  isLoading?: boolean;
}

// ✅ forwardRef for ref forwarding
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'primary', size = 'md', isLoading, children, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={`${styles.button} ${styles[variant]} ${styles[size]}`}
        disabled={isLoading || props.disabled}
        {...props}
      >
        {isLoading ? <Spinner size={size} /> : children}
      </button>
    );
  }
);
```

---

## Files Requiring Updates

### M11: React Foundation

#### 11.1.md - Project Setup
**Status:** Mostly OK (setup/config file)
**Changes needed:** None (just setup instructions)

#### 11.2.md - API Client
**Status:** Needs review
**Changes needed:**
- Update fetch wrapper to proper TypeScript patterns
- Add proper error handling conventions
- Use production-ready hook patterns

#### 11.3.md - Authentication UI
**Status:** Needs major updates
**Components to update:**
- `LoginPage` - Convert to production pattern
- `SetupPage` - Convert to production pattern
- Form inputs - Use reusable Input component with forwardRef

**Example fix:**
```tsx
// Current (prototype)
<input
  type="password"
  className="w-full px-3 py-2 border rounded..."
/>

// Production
<Input
  type="password"
  variant="outlined"
  size="md"
/>
```

#### 11.4.md - Layout Components
**Status:** Needs major updates
**Components to update:**
- `Layout` - Add CSS modules
- `Sidebar` - Convert to production component pattern
- `Header` - Convert to production component pattern
- `StatusDot` - Make reusable with proper typing

---

### M12: React Features

#### 12.1.md - Chat View
**Status:** Needs major updates
**Components to update:**
- `ChatPage` - Convert to use composition pattern
- `ChatInput` - forwardRef, extend textarea attributes, CSS modules
- `Message` - Component composition (MessageBubble, MessageAvatar, etc.)
- `MarkdownContent` - Proper production pattern
- `CodeBlock` - CSS modules, proper state management

**Missing cleanup:**
- ChatInput textarea needs proper cleanup
- Streaming subscription needs cleanup in useEffect

#### 12.2.md - Feed View
**Status:** Needs major updates
**Components to update:**
- `FeedPage` - Production pattern
- `FeedItem` - Component composition (FeedItemHeader, FeedItemContent, etc.)
- WebSocket subscription needs proper cleanup

#### 12.3.md - Tasks View
**Status:** Needs major updates
**Components to update:**
- `TasksPage` - Production pattern
- `TaskCard` - Use Card composition from CONVENTIONS.md
- `TaskDetail` - Modal/drawer with proper patterns
- Task status indicators - Reusable StatusBadge component

#### 12.4.md - Agents View
**Status:** Needs major updates
**Components to update:**
- `AgentsPage` - Production pattern
- `AgentCard` - Use Card composition
- Agent status - Reusable components

#### 12.5.md - File Browser & Editor
**Status:** Needs major updates
**Components to update:**
- `FileBrowser` - Production tree component
- `FileViewer` - Proper syntax highlighting integration
- `FileEditor` - Monaco/CodeMirror integration with proper cleanup

#### 12.6.md - Diff Viewer
**Status:** Needs updates
**Components to update:**
- `DiffViewer` - Production pattern with CSS modules

---

## Key Convention Violations in Decomp Files

### 1. No CSS Modules
**Current:** All styles are inline Tailwind
```tsx
className="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90"
```

**Should be:**
```tsx
// Button.module.css
.button { @apply px-4 py-2 rounded-lg transition-colors; }
.primary { @apply bg-accent text-white hover:bg-accent/90; }

// Button.tsx
className={`${styles.button} ${styles.primary}`}
```

### 2. No forwardRef
**Current:** Direct components
```tsx
export function Input({ value, onChange }: InputProps) {
```

**Should be:**
```tsx
export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ value, onChange, ...props }, ref) => {
```

### 3. Props Don't Extend HTML Attributes
**Current:**
```tsx
interface ButtonProps {
  onClick?: () => void;
  disabled?: boolean;
  className?: string;
}
```

**Should be:**
```tsx
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary';
  isLoading?: boolean;
}
// onClick, disabled, className, etc. are inherited
```

### 4. No Component Composition
**Current:** Monolithic components
```tsx
function TaskCard({ task }) {
  return (
    <div>
      <h3>{task.title}</h3>
      <p>{task.status}</p>
    </div>
  );
}
```

**Should be:**
```tsx
function TaskCard({ task }) {
  return (
    <Card>
      <CardHeader title={task.title} />
      <CardBody>
        <StatusBadge status={task.status} />
      </CardBody>
    </Card>
  );
}
```

### 5. Missing useEffect Cleanup
**Current:**
```tsx
useEffect(() => {
  const ws = new WebSocket('ws://localhost');
  ws.onmessage = (e) => setMessages([...messages, e.data]);
}, []);
```

**Should be:**
```tsx
useEffect(() => {
  const ws = new WebSocket('ws://localhost');
  ws.onmessage = (e) => setMessages((prev) => [...prev, e.data]);

  return () => {
    ws.close();
  };
}, []);
```

### 6. No Proper File Organization
**Current:**
```
ui/src/components/
  ├── Button.tsx
  ├── Input.tsx
```

**Should be:**
```
ui/src/components/
  ├── Button/
  │   ├── Button.tsx
  │   ├── Button.module.css
  │   ├── Button.test.tsx
  │   └── index.ts
  ├── Input/
      ├── Input.tsx
      ├── Input.module.css
      ├── Input.test.tsx
      └── index.ts
```

---

## Recommended Approach

### Option 1: Update All Decomp Files Now
**Pros:**
- Workers get correct patterns from the start
- No confusion about what to follow
- Consistency across all tickets

**Cons:**
- Large upfront work
- May delay M11/M12 implementation

### Option 2: Update on Demand
**Pros:**
- Can start M11/M12 immediately
- Workers reference CONVENTIONS.md directly

**Cons:**
- Risk of workers following decomp examples instead of conventions
- Inconsistency if some workers don't read conventions

### Option 3: Add Warning to Decomp Files
**Pros:**
- Quick fix
- Alerts workers to use CONVENTIONS.md

**Cons:**
- Still has bad examples in decomp files

---

## Recommendation

**Update all decomp files to production patterns** for the following reasons:

1. Prevents workers from copy-pasting prototype code
2. Ensures consistency from day one
3. Decomp files become reference implementations
4. Less technical debt later

The update can be done systematically:
- Create reusable base components (Button, Input, Card, etc.) in decomp examples
- Show proper component composition
- Add CSS module examples
- Include cleanup patterns in all useEffect examples

---

## Next Steps

1. **Decision:** Choose approach (Option 1, 2, or 3)
2. **If Option 1:** Update decomp files systematically (can parallelize)
3. **If Option 2/3:** Add prominent warning to each decomp file linking to CONVENTIONS.md
4. **Document:** Update CLAUDE.md to reference production conventions for M11/M12

