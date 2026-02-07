# React Flow v12+ Enterprise Reference

Comprehensive reference for building an enterprise-grade AI workflow builder with React Flow (`@xyflow/react`). Covers custom nodes, handles, drag-and-drop, theming, edge animations, layout algorithms, performance, interactivity, state management, validation, and built-in components.

All code patterns target TypeScript + React 19. Package: `@xyflow/react` (v12+).

---

## Table of Contents

1. [Custom Nodes](#1-custom-nodes)
2. [Custom Handles (Ports)](#2-custom-handles-ports)
3. [Drag and Drop](#3-drag-and-drop)
4. [Styling and Theming](#4-styling-and-theming)
5. [Edge Animations](#5-edge-animations)
6. [Edge Types](#6-edge-types)
7. [Layout Algorithms](#7-layout-algorithms)
8. [Performance](#8-performance)
9. [Interactivity](#9-interactivity)
10. [State Management](#10-state-management)
11. [Validation](#11-validation)
12. [Minimap, Controls, Background](#12-minimap-controls-background)
13. [React Flow Pro / React Flow UI](#13-react-flow-pro--react-flow-ui)
14. [Common Pitfalls](#14-common-pitfalls)
15. [ReactFlow Component Props Reference](#15-reactflow-component-props-reference)

---

## 1. Custom Nodes

### Core Pattern

Custom nodes are plain React components. React Flow wraps them with an interactive container that injects props (`id`, `data`, `position`, `selected`, `dragging`, etc.).

```tsx
import { Handle, Position, type NodeProps, type Node } from '@xyflow/react';

type WorkflowNodeData = {
  label: string;
  status: 'idle' | 'running' | 'success' | 'error';
  icon: string;
};

type WorkflowNode = Node<WorkflowNodeData, 'workflow'>;

function WorkflowNode({ id, data, selected }: NodeProps<WorkflowNode>) {
  return (
    <div className={`workflow-node ${data.status} ${selected ? 'selected' : ''}`}>
      <Handle type="target" position={Position.Left} />

      <div className="workflow-node__header">
        <span className="workflow-node__icon">{data.icon}</span>
        <span className="workflow-node__label">{data.label}</span>
      </div>

      <div className="workflow-node__status-indicator" />

      <Handle type="source" position={Position.Right} />
    </div>
  );
}
```

### Registering Node Types

**Always define `nodeTypes` outside the component body.** Defining it inside causes React Flow to remount all nodes on every render.

```tsx
import { type NodeTypes } from '@xyflow/react';

// OUTSIDE the component -- prevents re-renders
const nodeTypes: NodeTypes = {
  workflow: WorkflowNode,
  llmCall: LLMCallNode,
  condition: ConditionNode,
  trigger: TriggerNode,
  group: GroupNode,
} satisfies NodeTypes;

function Canvas() {
  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      // ...
    />
  );
}
```

### Using Nodes

```tsx
const nodes: Node[] = [
  {
    id: 'llm-1',
    type: 'workflow',
    position: { x: 200, y: 100 },
    data: {
      label: 'GPT-4 Summarize',
      status: 'idle',
      icon: 'brain',
    },
  },
];
```

### Interactive Elements Inside Nodes

Add the `nodrag` CSS class to any element that should not trigger node dragging (inputs, selects, text areas, sliders). Add `nowheel` to elements that should capture scroll without zooming the canvas.

```tsx
function LLMCallNode({ data }: NodeProps) {
  return (
    <div className="llm-node">
      <Handle type="target" position={Position.Top} />

      <textarea
        className="nodrag nowheel"
        defaultValue={data.prompt}
        rows={4}
      />

      <select className="nodrag" defaultValue={data.model}>
        <option value="gpt-4">GPT-4</option>
        <option value="claude-opus">Claude Opus</option>
      </select>

      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}
```

### Custom Shapes (Diamond, Hexagon, Circle)

Use SVG clip paths or shape components. Handles must be positioned relative to the shape.

```tsx
function DiamondNode({ data }: NodeProps) {
  return (
    <div className="diamond-node" style={{ width: 100, height: 100 }}>
      <svg viewBox="0 0 100 100" className="diamond-shape">
        <polygon points="50,0 100,50 50,100 0,50" fill="var(--node-bg)" stroke="var(--node-border)" />
      </svg>

      <div className="diamond-node__label">{data.label}</div>

      {/* Position handles at diamond vertices */}
      <Handle type="target" position={Position.Top} style={{ left: '50%', top: '-4px' }} />
      <Handle type="source" position={Position.Bottom} style={{ left: '50%', bottom: '-4px' }} />
      <Handle type="source" position={Position.Right} id="right" style={{ right: '-4px', top: '50%' }} />
      <Handle type="target" position={Position.Left} id="left" style={{ left: '-4px', top: '50%' }} />
    </div>
  );
}
```

### Node Width/Height in v12

v12 allows declaring `width` and `height` on the node object. This enables server-side rendering and layout without DOM measurement.

```tsx
const nodes: Node[] = [
  {
    id: '1',
    type: 'workflow',
    position: { x: 0, y: 0 },
    data: { label: 'Start' },
    width: 250,
    height: 80,
  },
];
```

---

## 2. Custom Handles (Ports)

### Handle Component Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `type` | `'source' \| 'target'` | `'source'` | Direction of connection |
| `position` | `Position` | `Position.Top` | Side of node |
| `id` | `string \| null` | `null` | Unique ID for multiple handles |
| `isConnectable` | `boolean \| number` | `true` | Enable/disable or max connections |
| `isConnectableStart` | `boolean` | `true` | Can initiate connections |
| `isConnectableEnd` | `boolean` | `true` | Can receive connections |
| `isValidConnection` | `(connection: Connection) => boolean` | - | Per-handle validation |
| `onConnect` | `(connection: Connection) => void` | - | Fires on successful connection |

### Positioning Multiple Handles

React Flow centers handles on the specified side by default. Use CSS to position multiple handles along an edge.

```tsx
function MultiPortNode({ data }: NodeProps) {
  return (
    <div className="multi-port-node">
      {/* Target handles -- stacked vertically on the left */}
      {data.inputs.map((input: { id: string; label: string }, i: number) => (
        <Handle
          key={input.id}
          type="target"
          position={Position.Left}
          id={input.id}
          style={{ top: `${((i + 1) / (data.inputs.length + 1)) * 100}%` }}
        />
      ))}

      <div className="node-body">{data.label}</div>

      {/* Source handles -- stacked vertically on the right */}
      {data.outputs.map((output: { id: string; label: string }, i: number) => (
        <Handle
          key={output.id}
          type="source"
          position={Position.Right}
          id={output.id}
          style={{ top: `${((i + 1) / (data.outputs.length + 1)) * 100}%` }}
        />
      ))}
    </div>
  );
}
```

### Handles Inside or Outside the Node Box

Position handles that "hang" off the edge of the node or sit fully inside:

```css
/* Handle sitting ON the edge (default behavior) */
.react-flow__handle {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--handle-color);
  border: 2px solid var(--handle-border);
}

/* Handle fully INSIDE the node */
.react-flow__handle.inside-handle {
  width: 8px;
  height: 8px;
  transform: translate(4px, 0); /* Pull inward from edge */
}

/* Handle OUTSIDE the node with offset */
.react-flow__handle.outside-handle {
  width: 10px;
  height: 10px;
  transform: translate(-6px, 0); /* Push outward from edge */
}
```

### Custom Handle Component (Wrapping Any Element)

Wrap any React component as a Handle by hiding the default appearance:

```tsx
function TypedHandle({ type, position, id, handleType, color }: {
  type: 'source' | 'target';
  position: Position;
  id: string;
  handleType: string;
  color: string;
}) {
  return (
    <Handle
      type={type}
      position={position}
      id={id}
      style={{
        background: 'none',
        border: 'none',
        width: 16,
        height: 16,
      }}
    >
      {/* Custom visual -- pointer-events: none so Handle captures interactions */}
      <div
        style={{
          width: 12,
          height: 12,
          borderRadius: handleType === 'string' ? '50%' : 2,
          background: color,
          pointerEvents: 'none',
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
        }}
      />
    </Handle>
  );
}
```

### Dynamic Handle Counts

When handles are added/removed programmatically, call `useUpdateNodeInternals` to recalculate positions:

```tsx
import { useUpdateNodeInternals } from '@xyflow/react';

function DynamicHandleNode({ id, data }: NodeProps) {
  const updateNodeInternals = useUpdateNodeInternals();

  const addOutput = () => {
    // Update data.outputs in your state store
    // Then notify React Flow:
    updateNodeInternals(id);
  };

  return (
    <div>
      {data.outputs.map((out: { id: string }) => (
        <Handle key={out.id} type="source" position={Position.Right} id={out.id} />
      ))}
      <button className="nodrag" onClick={addOutput}>+ Add Output</button>
    </div>
  );
}
```

### Connection Limit Per Handle

Use `useNodeConnections` to dynamically compute `isConnectable`:

```tsx
import { Handle, useNodeConnections, type HandleProps } from '@xyflow/react';

type LimitedHandleProps = HandleProps & { connectionCount: number };

function LimitedHandle({ connectionCount, ...props }: LimitedHandleProps) {
  const connections = useNodeConnections({ handleType: props.type });

  return (
    <Handle
      {...props}
      isConnectable={connections.length < connectionCount}
    />
  );
}

// Usage: only 1 incoming connection allowed
<LimitedHandle type="target" position={Position.Left} connectionCount={1} />
```

### Styling Handles During Connection

Handles receive CSS classes `connecting` and `valid` during a drag-to-connect interaction:

```css
.react-flow__handle.connecting {
  background: var(--color-warning);
}

.react-flow__handle.valid {
  background: var(--color-success);
}
```

**Important:** Never use `display: none` on handles. Use `visibility: hidden` or `opacity: 0` -- `display: none` prevents dimension calculation.

---

## 3. Drag and Drop

### Architecture (Pointer Events API)

The recommended approach uses the Pointer Events API for cross-device (mouse + touch) support. Three components: `DnDProvider` (context), `Sidebar` (palette), and the main flow.

### DnD Context and Hook

```tsx
import { useReactFlow, type XYPosition } from '@xyflow/react';
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';

type OnDropAction = (params: { position: XYPosition }) => void;

type DnDContextValue = {
  isDragging: boolean;
  setIsDragging: Dispatch<SetStateAction<boolean>>;
  dropAction: OnDropAction | null;
  setDropAction: Dispatch<SetStateAction<OnDropAction | null>>;
};

const DnDContext = createContext<DnDContextValue | null>(null);

function DnDProvider({ children }: { children: ReactNode }) {
  const [isDragging, setIsDragging] = useState(false);
  const [dropAction, setDropAction] = useState<OnDropAction | null>(null);

  return (
    <DnDContext.Provider
      value={{
        isDragging,
        setIsDragging,
        dropAction,
        setDropAction: (action) => setDropAction(() => action),
      }}
    >
      {children}
    </DnDContext.Provider>
  );
}

const useDnD = () => {
  const { screenToFlowPosition } = useReactFlow();
  const context = useContext(DnDContext);
  if (!context) throw new Error('useDnD must be used within DnDProvider');

  const { isDragging, setIsDragging, setDropAction, dropAction } = context;

  const onDragStart = useCallback(
    (event: React.PointerEvent<HTMLDivElement>, onDrop: OnDropAction) => {
      event.preventDefault();
      (event.target as HTMLElement).setPointerCapture(event.pointerId);
      setIsDragging(true);
      setDropAction(onDrop);
    },
    [setIsDragging, setDropAction],
  );

  const onDragEnd = useCallback(
    (event: PointerEvent) => {
      if (!isDragging) {
        setIsDragging(false);
        return;
      }
      (event.target as HTMLElement).releasePointerCapture(event.pointerId);
      const elementUnderPointer = document.elementFromPoint(event.clientX, event.clientY);
      const isDroppingOnFlow = elementUnderPointer?.closest('.react-flow');
      event.preventDefault();

      if (isDroppingOnFlow) {
        const flowPosition = screenToFlowPosition({
          x: event.clientX,
          y: event.clientY,
        });
        dropAction?.({ position: flowPosition });
      }
      setIsDragging(false);
    },
    [screenToFlowPosition, setIsDragging, dropAction, isDragging],
  );

  useEffect(() => {
    if (!isDragging) return;
    document.addEventListener('pointerup', onDragEnd);
    return () => document.removeEventListener('pointerup', onDragEnd);
  }, [onDragEnd, isDragging]);

  return { isDragging, onDragStart };
};

const useDnDPosition = () => {
  const [position, setPosition] = useState<XYPosition | null>(null);

  useEffect(() => {
    const onDrag = (event: PointerEvent) => {
      event.preventDefault();
      setPosition({ x: event.clientX, y: event.clientY });
    };
    document.addEventListener('pointermove', onDrag);
    return () => document.removeEventListener('pointermove', onDrag);
  }, []);

  return { position };
};
```

### Sidebar Palette

```tsx
function Sidebar() {
  const { onDragStart, isDragging } = useDnD();
  const [dragType, setDragType] = useState<string | null>(null);
  const { setNodes } = useReactFlow();

  const createAddNode = useCallback(
    (nodeType: string): OnDropAction => {
      return ({ position }) => {
        const newNode: Node = {
          id: crypto.randomUUID(),
          type: nodeType,
          position,
          data: { label: `New ${nodeType}` },
        };
        setNodes((nds) => nds.concat(newNode));
        setDragType(null);
      };
    },
    [setNodes],
  );

  const nodeTemplates = [
    { type: 'llmCall', label: 'LLM Call', icon: 'brain' },
    { type: 'condition', label: 'Condition', icon: 'split' },
    { type: 'trigger', label: 'Trigger', icon: 'zap' },
    { type: 'transform', label: 'Transform', icon: 'code' },
  ];

  return (
    <>
      {isDragging && <DragGhost type={dragType} />}
      <aside className="sidebar">
        {nodeTemplates.map((tpl) => (
          <div
            key={tpl.type}
            className="sidebar__item"
            onPointerDown={(e) => {
              setDragType(tpl.type);
              onDragStart(e, createAddNode(tpl.type));
            }}
          >
            <span>{tpl.icon}</span>
            <span>{tpl.label}</span>
          </div>
        ))}
      </aside>
    </>
  );
}
```

### Ghost Preview

```tsx
function DragGhost({ type }: { type: string | null }) {
  const { position } = useDnDPosition();
  if (!position || !type) return null;

  return (
    <div
      className="drag-ghost"
      style={{
        transform: `translate(${position.x}px, ${position.y}px) translate(-50%, -50%)`,
        position: 'fixed',
        pointerEvents: 'none',
        zIndex: 9999,
      }}
    >
      {type}
    </div>
  );
}
```

### Snap to Grid

Enable on the `<ReactFlow>` component:

```tsx
<ReactFlow
  snapToGrid={true}
  snapGrid={[20, 20]}  // 20px grid
  // ...
/>
```

### Assembly

```tsx
function App() {
  return (
    <ReactFlowProvider>
      <DnDProvider>
        <div className="app-layout">
          <Sidebar />
          <Canvas />
        </div>
      </DnDProvider>
    </ReactFlowProvider>
  );
}
```

---

## 4. Styling and Theming

### CSS Variable Reference

All variables use `--xy-` prefix with `-default` suffix. Override under `.react-flow`:

```css
.react-flow {
  /* Edges */
  --xy-edge-stroke-default: #b1b1b7;
  --xy-edge-stroke-width-default: 1;
  --xy-edge-stroke-selected-default: #555;
  --xy-connectionline-stroke-default: #b1b1b7;
  --xy-connectionline-stroke-width-default: 1;

  /* Nodes */
  --xy-node-color-default: inherit;
  --xy-node-border-default: 1px solid #1a192b;
  --xy-node-background-color-default: #fff;
  --xy-node-group-background-color-default: rgba(240, 240, 240, 0.25);
  --xy-node-boxshadow-hover-default: 0 1px 4px 1px rgba(0, 0, 0, 0.08);
  --xy-node-boxshadow-selected-default: 0 0 0 0.5px #1a192b;

  /* Handles */
  --xy-handle-background-color-default: #1a192b;
  --xy-handle-border-color-default: #fff;

  /* Controls */
  --xy-controls-button-background-color-default: #fefefe;
  --xy-controls-button-background-color-hover-default: #f4f4f4;
  --xy-controls-box-shadow-default: 0 0 2px 1px rgba(0, 0, 0, 0.08);

  /* Selection */
  --xy-selection-background-color-default: rgba(0, 89, 220, 0.08);
  --xy-selection-border-default: 1px dotted rgba(0, 89, 220, 0.8);

  /* MiniMap */
  --xy-minimap-background-color-default: #fff;

  /* Background patterns */
  --xy-background-pattern-dots-color-default: #91919a;
  --xy-background-pattern-line-color-default: #eee;
  --xy-background-pattern-cross-color-default: #e2e2e2;

  /* Resize */
  --xy-resize-background-color-default: #3367d9;
}
```

### Built-In Dark Mode

The `colorMode` prop handles dark/light/system. It adds a `.dark` or `.light` class to the wrapper.

```tsx
import { useState, type ColorMode } from '@xyflow/react';

function Canvas() {
  const [colorMode, setColorMode] = useState<ColorMode>('dark');

  return (
    <ReactFlow colorMode={colorMode} nodes={nodes} edges={edges}>
      <MiniMap />
      <Background />
      <Controls />
    </ReactFlow>
  );
}
```

### Custom Dark Theme Override

```css
.dark .react-flow {
  --xy-node-background-color-default: #1e1e2e;
  --xy-node-border-default: 1px solid rgba(255, 255, 255, 0.08);
  --xy-node-color-default: #cdd6f4;
  --xy-node-boxshadow-selected-default: 0 0 0 2px #89b4fa;
  --xy-node-boxshadow-hover-default: 0 2px 8px rgba(0, 0, 0, 0.3);

  --xy-edge-stroke-default: #585b70;
  --xy-edge-stroke-selected-default: #89b4fa;

  --xy-handle-background-color-default: #89b4fa;
  --xy-handle-border-color-default: #1e1e2e;

  --xy-minimap-background-color-default: #11111b;
  --xy-background-pattern-dots-color-default: #313244;

  --xy-controls-button-background-color-default: #1e1e2e;
  --xy-controls-button-background-color-hover-default: #313244;
}
```

### Glassmorphism / Modern Node Style

```css
.glass-node {
  background: rgba(30, 30, 46, 0.6);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  box-shadow:
    0 4px 24px rgba(0, 0, 0, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
  padding: 16px;
  color: #cdd6f4;
  transition: box-shadow 0.2s ease, border-color 0.2s ease;
}

.glass-node:hover {
  border-color: rgba(137, 180, 250, 0.3);
  box-shadow:
    0 4px 24px rgba(0, 0, 0, 0.3),
    0 0 0 1px rgba(137, 180, 250, 0.1);
}

.glass-node.selected {
  border-color: #89b4fa;
  box-shadow:
    0 0 0 2px rgba(137, 180, 250, 0.3),
    0 4px 24px rgba(0, 0, 0, 0.3);
}
```

### Tailwind CSS Integration

Import React Flow styles before Tailwind to allow proper cascade:

```tsx
import '@xyflow/react/dist/style.css';
import './tailwind.css'; // Tailwind after React Flow
```

Or for maximum control, import only base styles:

```tsx
import '@xyflow/react/dist/base.css'; // Required for functionality
import './tailwind.css';
// ... then apply your own styles for everything
```

---

## 5. Edge Animations

### Approach 1: SVG animateMotion (Recommended for Performance)

Animate a shape traveling along the edge path. **Much better performance than stroke-dasharray** -- frame drops reduced from ~5 frames to 2-3 frames per observation.

```tsx
import { BaseEdge, getSmoothStepPath, type EdgeProps } from '@xyflow/react';

function AnimatedSVGEdge({
  id, sourceX, sourceY, targetX, targetY,
  sourcePosition, targetPosition,
}: EdgeProps) {
  const [edgePath] = getSmoothStepPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
  });

  return (
    <>
      <BaseEdge id={id} path={edgePath} />
      {/* Traveling circle -- "particle flow" effect */}
      <circle r="4" fill="#89b4fa">
        <animateMotion dur="2s" repeatCount="indefinite" path={edgePath} />
      </circle>
    </>
  );
}
```

### Approach 2: Multiple Particles

```tsx
function MultiParticleEdge({ id, ...props }: EdgeProps) {
  const [edgePath] = getSmoothStepPath(props);

  return (
    <>
      <BaseEdge id={id} path={edgePath} style={{ stroke: '#585b70' }} />
      {[0, 0.33, 0.66].map((delay, i) => (
        <circle key={i} r="3" fill="#a6e3a1" opacity="0.8">
          <animateMotion
            dur="3s"
            repeatCount="indefinite"
            path={edgePath}
            begin={`${delay * 3}s`}
          />
        </circle>
      ))}
    </>
  );
}
```

### Approach 3: Gradient Pulse

```tsx
function GradientPulseEdge({ id, ...props }: EdgeProps) {
  const [edgePath] = getBezierPath(props);
  const gradientId = `gradient-${id}`;

  return (
    <>
      <defs>
        <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#89b4fa" stopOpacity="0">
            <animate attributeName="offset" values="-0.5;1" dur="2s" repeatCount="indefinite" />
          </stop>
          <stop offset="50%" stopColor="#89b4fa" stopOpacity="1">
            <animate attributeName="offset" values="0;1.5" dur="2s" repeatCount="indefinite" />
          </stop>
          <stop offset="100%" stopColor="#89b4fa" stopOpacity="0">
            <animate attributeName="offset" values="0.5;2" dur="2s" repeatCount="indefinite" />
          </stop>
        </linearGradient>
      </defs>
      <BaseEdge id={id} path={edgePath} style={{ stroke: `url(#${gradientId})`, strokeWidth: 2 }} />
    </>
  );
}
```

### Approach 4: Animated Node Along Edge (Web Animations API)

Use CSS `offsetPath` to animate a React Flow node along an edge path:

```tsx
function AnimatedNodeEdge({
  id, data, sourceX, sourceY, targetX, targetY,
  sourcePosition, targetPosition,
}: EdgeProps<{ node: string }>) {
  const { getNode, updateNode } = useReactFlow();
  const [edgePath] = getBezierPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
  });

  const selector = useMemo(
    () => `.react-flow__node[data-id="${data?.node}"]`,
    [data?.node],
  );

  useEffect(() => {
    const el = document.querySelector(selector) as HTMLElement;
    if (!el || !data?.node) return;

    el.style.offsetPath = `path('${edgePath}')`;
    el.style.offsetRotate = '0deg';
    el.style.offsetAnchor = 'center';

    const wasDraggable = getNode(data.node)?.draggable;
    updateNode(data.node, { draggable: false });

    return () => {
      el.style.offsetPath = 'none';
      updateNode(data.node, { draggable: wasDraggable });
    };
  }, [selector, edgePath, data?.node, getNode, updateNode]);

  useEffect(() => {
    const el = document.querySelector(selector) as HTMLElement;
    if (!el) return;

    const animation = el.animate(
      [{ offsetDistance: '0%' }, { offsetDistance: '100%' }],
      { duration: 2000, direction: 'alternate', iterations: Infinity },
    );

    return () => animation.cancel();
  }, [selector]);

  return <BaseEdge id={id} path={edgePath} />;
}
```

### Performance: stroke-dasharray vs animateMotion

**Avoid `animated: true` on edges for large graphs.** It uses `stroke-dasharray` CSS animation which is CPU-intensive. Benchmarks show:

| Technique | Frame Drop (100 edges) |
|-----------|----------------------|
| `stroke-dasharray` | ~5 frames per interaction |
| `animateMotion` | ~2-3 frames per interaction |
| No animation | ~0-1 frames |

**Recommendation:** Use `animateMotion` for flow-direction indicators. Limit to visible edges. Remove animations entirely during drag operations for maximum responsiveness.

---

## 6. Edge Types

### Built-In Types

| Type | Description | Use Case |
|------|-------------|----------|
| `default` (bezier) | Curved path | General purpose |
| `straight` | Direct line | Minimal UIs |
| `step` | Right-angle turns | Circuit/process diagrams |
| `smoothstep` | Rounded right-angle | Workflow builders (recommended) |

### Path Helper Functions

```tsx
import {
  getBezierPath,
  getStraightPath,
  getSimpleBezierPath,
  getSmoothStepPath,
} from '@xyflow/react';
```

All return `[edgePath, labelX, labelY, offsetX, offsetY]`.

### Custom Edge with Delete Button

```tsx
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  useReactFlow,
  type EdgeProps,
} from '@xyflow/react';

function ButtonEdge({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }: EdgeProps) {
  const { setEdges } = useReactFlow();
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
  });

  const onDelete = () => {
    setEdges((edges) => edges.filter((e) => e.id !== id));
  };

  return (
    <>
      <BaseEdge id={id} path={edgePath} />
      <EdgeLabelRenderer>
        <div
          className="nodrag nopan"
          style={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
            pointerEvents: 'all',
          }}
        >
          <button className="edge-delete-btn" onClick={onDelete}>x</button>
        </div>
      </EdgeLabelRenderer>
    </>
  );
}
```

### Bidirectional Edge

Detect reverse edges and offset the path to avoid overlap:

```tsx
function BiDirectionalEdge(props: EdgeProps) {
  const { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props;

  // Check for reverse edge
  const edges = useStore((s) => s.edges);
  const hasReverse = edges.some(
    (e) => e.source === props.target && e.target === props.source,
  );

  if (hasReverse) {
    // Offset the curve to avoid overlap
    const offset = sourceY < targetY ? -30 : 30;
    const [edgePath] = getBezierPath({
      sourceX, sourceY: sourceY + offset, sourcePosition,
      targetX, targetY: targetY + offset, targetPosition,
    });
    return <BaseEdge id={props.id} path={edgePath} />;
  }

  const [edgePath] = getBezierPath({ sourceX, sourceY, sourcePosition, targetX, targetY, targetPosition });
  return <BaseEdge id={props.id} path={edgePath} />;
}
```

### Self-Connecting Edge

```tsx
function SelfConnectingEdge(props: EdgeProps) {
  if (props.source !== props.target) {
    return <BezierEdge {...props} />;
  }

  const { sourceX, sourceY, targetX, targetY } = props;
  const radiusX = (sourceX - targetX) * 0.6;
  const radiusY = 50;
  const edgePath = `M ${sourceX - 5} ${sourceY} A ${radiusX} ${radiusY} 0 1 0 ${targetX + 2} ${targetY}`;

  return <BaseEdge path={edgePath} id={props.id} />;
}
```

### Edge Markers (Arrows)

```tsx
const edges: Edge[] = [
  {
    id: 'e1-2',
    source: '1',
    target: '2',
    markerEnd: { type: MarkerType.ArrowClosed, color: '#89b4fa' },
    // Or for start:
    // markerStart: { type: MarkerType.Arrow },
  },
];

// Default for all edges:
<ReactFlow
  defaultEdgeOptions={{
    type: 'smoothstep',
    markerEnd: { type: MarkerType.ArrowClosed },
    animated: false,
  }}
/>
```

### Register Custom Edge Types

```tsx
const edgeTypes = {
  animated: AnimatedSVGEdge,
  button: ButtonEdge,
  bidirectional: BiDirectionalEdge,
  selfconnecting: SelfConnectingEdge,
} satisfies EdgeTypes;

// DEFINE OUTSIDE COMPONENT
```

---

## 7. Layout Algorithms

### Overview

React Flow has no built-in layout. Use external libraries:

| Library | Strengths | Weaknesses |
|---------|-----------|------------|
| **dagre** (`@dagrejs/dagre`) | Simple, fast, tree-focused | Deprecated, no async, fixed node sizes |
| **ELK** (`elkjs`) | Hugely configurable, edge routing, async | Complex API, heavier bundle |
| **d3-hierarchy** | Great for strict trees | Single-root only, uniform node sizes |
| **d3-force** | Organic/physics layouts | Iterative, requires multiple renders |

### Dagre Layout

```tsx
import dagre from '@dagrejs/dagre';
import { Position, type Node, type Edge } from '@xyflow/react';

const NODE_WIDTH = 250;
const NODE_HEIGHT = 80;

const getLayoutedElements = (nodes: Node[], edges: Edge[], direction = 'TB') => {
  const dagreGraph = new dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
  const isHorizontal = direction === 'LR';

  dagreGraph.setGraph({ rankdir: direction, nodesep: 50, ranksep: 80 });

  nodes.forEach((node) => {
    dagreGraph.setNode(node.id, {
      width: node.width ?? NODE_WIDTH,
      height: node.height ?? NODE_HEIGHT,
    });
  });

  edges.forEach((edge) => {
    dagreGraph.setEdge(edge.source, edge.target);
  });

  dagre.layout(dagreGraph);

  const layoutedNodes = nodes.map((node) => {
    const dagreNode = dagreGraph.node(node.id);
    return {
      ...node,
      position: {
        x: dagreNode.x - (node.width ?? NODE_WIDTH) / 2,
        y: dagreNode.y - (node.height ?? NODE_HEIGHT) / 2,
      },
      targetPosition: isHorizontal ? Position.Left : Position.Top,
      sourcePosition: isHorizontal ? Position.Right : Position.Bottom,
    };
  });

  return { nodes: layoutedNodes, edges };
};
```

### ELK Layout

```tsx
import ELK from 'elkjs/lib/elk.bundled.js';

const elk = new ELK();

const elkOptions = {
  'elk.algorithm': 'layered',
  'elk.layered.spacing.nodeNodeBetweenLayers': '100',
  'elk.spacing.nodeNode': '80',
};

const getLayoutedElements = async (
  nodes: Node[],
  edges: Edge[],
  options: Record<string, string> = {},
) => {
  const isHorizontal = options['elk.direction'] === 'RIGHT';

  const graph = {
    id: 'root',
    layoutOptions: { ...elkOptions, ...options },
    children: nodes.map((node) => ({
      ...node,
      targetPosition: isHorizontal ? 'left' : 'top',
      sourcePosition: isHorizontal ? 'right' : 'bottom',
      width: node.width ?? 250,
      height: node.height ?? 80,
    })),
    edges: edges.map((e) => ({
      id: e.id,
      sources: [e.source],
      targets: [e.target],
    })),
  };

  const layoutedGraph = await elk.layout(graph);

  return {
    nodes: (layoutedGraph.children ?? []).map((node) => ({
      ...node,
      position: { x: node.x!, y: node.y! },
    })),
    edges,
  };
};
```

### ELK with Multiple Handles (Port Constraints)

```tsx
const elkOptionsWithPorts = {
  'elk.algorithm': 'layered',
  'elk.direction': 'RIGHT',
  'elk.layered.spacing.edgeNodeBetweenLayers': '40',
  'elk.spacing.nodeNode': '40',
  'elk.layered.nodePlacement.strategy': 'SIMPLE',
};

// On each node, add port constraints:
const elkNode = {
  id: node.id,
  width: 250,
  height: 100,
  properties: {
    'org.eclipse.elk.portConstraints': 'FIXED_ORDER',
  },
  ports: [
    ...node.data.inputs.map((input: { id: string }) => ({
      id: input.id,
      properties: { side: 'WEST' },
    })),
    ...node.data.outputs.map((output: { id: string }) => ({
      id: output.id,
      properties: { side: 'EAST' },
    })),
  ],
};
```

### Auto-Layout Hook Pattern

```tsx
import { useCallback } from 'react';
import { useReactFlow, useNodesInitialized } from '@xyflow/react';

const useAutoLayout = (direction: 'TB' | 'LR' = 'TB') => {
  const { getNodes, getEdges, setNodes, setEdges, fitView } = useReactFlow();
  const nodesInitialized = useNodesInitialized();

  const runLayout = useCallback(async () => {
    const nodes = getNodes();
    const edges = getEdges();

    if (nodes.length === 0) return;

    const { nodes: layoutedNodes, edges: layoutedEdges } =
      await getLayoutedElements(nodes, edges, { 'elk.direction': direction === 'LR' ? 'RIGHT' : 'DOWN' });

    setNodes(layoutedNodes);
    setEdges(layoutedEdges);

    // Wait for React to render, then fit
    requestAnimationFrame(() => fitView({ padding: 0.2 }));
  }, [getNodes, getEdges, setNodes, setEdges, fitView, direction]);

  // Auto-run when nodes are first measured
  useEffect(() => {
    if (nodesInitialized) {
      runLayout();
    }
  }, [nodesInitialized, runLayout]);

  return { runLayout };
};
```

### Layout Tips

- **Measured vs computed dimensions:** Dagre and d3-hierarchy require knowing node sizes up front. Either declare `width`/`height` on nodes or use `useNodesInitialized` to wait for DOM measurement.
- **Toggle layout on/off:** For large graphs, do not recompute layout on every change. Provide a "Re-layout" button.
- **Sub-flow limitation with dagre:** dagre has an open issue where sub-flow nodes connected externally break the layout. Use ELK for complex nested graphs.
- **Start simple:** Use dagre first, escalate to ELK only when you need edge routing, port constraints, or complex configurations.

---

## 8. Performance

### Memoization (Critical)

```tsx
import { memo, useCallback, useMemo } from 'react';

// Memoize custom node components
const WorkflowNode = memo(function WorkflowNode({ data }: NodeProps) {
  return <div>{data.label}</div>;
});

// Memoize all callback props
function Canvas() {
  const onConnect = useCallback((params: Connection) => {
    setEdges((eds) => addEdge(params, eds));
  }, [setEdges]);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setNodes((nds) => applyNodeChanges(changes, nds));
  }, [setNodes]);

  // Memoize config objects
  const defaultEdgeOptions = useMemo(() => ({
    type: 'smoothstep',
    markerEnd: { type: MarkerType.ArrowClosed },
  }), []);

  const snapGrid = useMemo<[number, number]>(() => [20, 20], []);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      onConnect={onConnect}
      onNodesChange={onNodesChange}
      defaultEdgeOptions={defaultEdgeOptions}
      snapGrid={snapGrid}
      // ...
    />
  );
}
```

### Avoid Direct Node/Edge Access

The single biggest performance killer is subscribing to the full nodes/edges arrays in components:

```tsx
// BAD -- re-renders on every node position change
const nodes = useStore((state) => state.nodes);
const selectedIds = nodes.filter((n) => n.selected).map((n) => n.id);

// GOOD -- surgical selector that only changes when selection changes
const selectedIds = useStore(
  useCallback(
    (state) => state.nodes.filter((n) => n.selected).map((n) => n.id),
    [],
  ),
);

// BEST -- track selection separately in your own store
const selectedIds = useWorkflowStore((s) => s.selectedNodeIds);
```

### Viewport Culling

Enable `onlyRenderVisibleElements` to skip rendering nodes outside the viewport:

```tsx
<ReactFlow onlyRenderVisibleElements={true} />
```

### Collapse Large Subtrees

Toggle `hidden` on nodes rather than rendering everything:

```tsx
const toggleSubtree = (parentId: string) => {
  setNodes((nodes) =>
    nodes.map((node) => {
      if (node.parentId === parentId) {
        return { ...node, hidden: !node.hidden };
      }
      return node;
    }),
  );
};
```

### CSS Performance

Avoid on large graphs:
- `box-shadow` with large spread
- `backdrop-filter: blur()`
- `stroke-dasharray` animations
- CSS transitions on nodes during drag

Use simpler styles during interactions:

```css
/* Disable expensive effects while dragging */
.react-flow__node.dragging .glass-node {
  backdrop-filter: none;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  transition: none;
}
```

### Event Throttling

Throttle expensive operations during drag:

```tsx
import { throttle } from 'lodash-es';

const onNodeDrag = useMemo(
  () => throttle((_event: React.MouseEvent, node: Node) => {
    // Expensive computation like collision detection
  }, 100),
  [],
);
```

### Large Graph Strategy (1000+ Nodes)

1. Use `onlyRenderVisibleElements`
2. Memo all custom node/edge components
3. Avoid `animated: true` on edges
4. Collapse subtrees / use hierarchical expand
5. Debounce layout recalculations
6. Use Zustand selectors to avoid broad re-renders
7. Profile with React DevTools -- look for unnecessary re-renders

---

## 9. Interactivity

### Multi-Select

Built-in: hold **Shift + click** to add to selection, or **Shift + drag** for selection box.

```tsx
<ReactFlow
  selectionKeyCode="Shift"       // Selection box key
  multiSelectionKeyCode="Meta"   // Cmd/Ctrl click to toggle
  deleteKeyCode="Backspace"      // Delete selected
  selectionOnDrag={false}        // Set true to drag-select without Shift
  panOnDrag={[1, 2]}             // Middle/right mouse for pan when selectionOnDrag=true
/>
```

### Context Menu

```tsx
function Canvas() {
  const [menu, setMenu] = useState<{ id: string; x: number; y: number } | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  const onNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: Node) => {
      event.preventDefault();

      if (!ref.current) return;
      const pane = ref.current.getBoundingClientRect();

      setMenu({
        id: node.id,
        x: Math.min(event.clientX - pane.left, pane.width - 200),
        y: Math.min(event.clientY - pane.top, pane.height - 200),
      });
    },
    [],
  );

  const onPaneClick = useCallback(() => setMenu(null), []);

  return (
    <div ref={ref}>
      <ReactFlow
        onNodeContextMenu={onNodeContextMenu}
        onPaneClick={onPaneClick}
      >
        {menu && (
          <ContextMenu
            id={menu.id}
            x={menu.x}
            y={menu.y}
            onClose={() => setMenu(null)}
          />
        )}
      </ReactFlow>
    </div>
  );
}

function ContextMenu({ id, x, y, onClose }: {
  id: string; x: number; y: number; onClose: () => void;
}) {
  const { getNode, setNodes, addNodes, setEdges } = useReactFlow();

  const duplicate = () => {
    const node = getNode(id);
    if (!node) return;
    addNodes({
      ...node,
      id: crypto.randomUUID(),
      position: { x: node.position.x + 50, y: node.position.y + 50 },
      selected: false,
    });
    onClose();
  };

  const remove = () => {
    setNodes((nds) => nds.filter((n) => n.id !== id));
    setEdges((eds) => eds.filter((e) => e.source !== id && e.target !== id));
    onClose();
  };

  return (
    <div className="context-menu" style={{ position: 'absolute', top: y, left: x, zIndex: 10 }}>
      <button onClick={duplicate}>Duplicate</button>
      <button onClick={remove}>Delete</button>
    </div>
  );
}
```

### Keyboard Shortcuts

Use `useKeyPress` hook from React Flow or roll your own:

```tsx
import { useKeyPress } from '@xyflow/react';

function Canvas() {
  const deletePressed = useKeyPress('Backspace');
  const selectAllPressed = useKeyPress(['Meta+a', 'Control+a']);

  // Or custom handler:
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          redo();
        } else {
          undo();
        }
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [undo, redo]);
}
```

### Node Grouping / Sub-Flows

```tsx
const nodes: Node[] = [
  // Group container
  {
    id: 'group-a',
    type: 'group',
    position: { x: 100, y: 100 },
    style: { width: 400, height: 300 },
    data: { label: 'LLM Pipeline' },
  },
  // Children -- parentId + relative position
  {
    id: 'step-1',
    type: 'workflow',
    position: { x: 20, y: 50 },  // Relative to parent
    parentId: 'group-a',
    extent: 'parent',             // Cannot be dragged outside
    data: { label: 'Summarize' },
  },
  {
    id: 'step-2',
    type: 'workflow',
    position: { x: 200, y: 50 },
    parentId: 'group-a',
    extent: 'parent',
    data: { label: 'Classify' },
  },
  // Nested group
  {
    id: 'group-b',
    type: 'group',
    position: { x: 20, y: 150 },
    parentId: 'group-a',
    style: { width: 360, height: 120, backgroundColor: 'rgba(137, 180, 250, 0.1)' },
    data: { label: 'Validation Sub-Flow' },
  },
  {
    id: 'validate-1',
    type: 'workflow',
    position: { x: 15, y: 40 },
    parentId: 'group-b',
    data: { label: 'Schema Check' },
  },
];
```

**Critical rule:** Parent nodes must appear before their children in the array.

### Collapsible Groups

```tsx
function GroupNode({ id, data }: NodeProps) {
  const { setNodes } = useReactFlow();
  const [collapsed, setCollapsed] = useState(false);

  const toggleCollapse = () => {
    const next = !collapsed;
    setCollapsed(next);
    setNodes((nodes) =>
      nodes.map((n) => {
        if (n.parentId === id) {
          return { ...n, hidden: next };
        }
        return n;
      }),
    );
  };

  return (
    <div className="group-node">
      <div className="group-node__header">
        <span>{data.label}</span>
        <button className="nodrag" onClick={toggleCollapse}>
          {collapsed ? 'Expand' : 'Collapse'}
        </button>
      </div>
      {!collapsed && <div className="group-node__body" />}
    </div>
  );
}
```

---

## 10. State Management

### Recommended: Zustand Store

React Flow uses Zustand internally and recommends it for external state. This pattern decouples node/edge state from component props.

```tsx
import { create } from 'zustand';
import {
  addEdge,
  applyNodeChanges,
  applyEdgeChanges,
  type Node,
  type Edge,
  type OnNodesChange,
  type OnEdgesChange,
  type OnConnect,
  type Connection,
} from '@xyflow/react';

type WorkflowState = {
  nodes: Node[];
  edges: Edge[];
  selectedNodeId: string | null;

  // React Flow handlers
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: OnConnect;

  // Custom actions
  setNodes: (nodes: Node[]) => void;
  setEdges: (edges: Edge[]) => void;
  addNode: (node: Node) => void;
  removeNode: (id: string) => void;
  updateNodeData: (id: string, data: Partial<Record<string, unknown>>) => void;
  setSelectedNodeId: (id: string | null) => void;
};

const useWorkflowStore = create<WorkflowState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,

  onNodesChange: (changes) => {
    set({ nodes: applyNodeChanges(changes, get().nodes) });
  },

  onEdgesChange: (changes) => {
    set({ edges: applyEdgeChanges(changes, get().edges) });
  },

  onConnect: (connection: Connection) => {
    set({ edges: addEdge(connection, get().edges) });
  },

  setNodes: (nodes) => set({ nodes }),
  setEdges: (edges) => set({ edges }),

  addNode: (node) => set({ nodes: [...get().nodes, node] }),

  removeNode: (id) => {
    set({
      nodes: get().nodes.filter((n) => n.id !== id),
      edges: get().edges.filter((e) => e.source !== id && e.target !== id),
    });
  },

  updateNodeData: (id, data) => {
    set({
      nodes: get().nodes.map((node) => {
        if (node.id !== id) return node;
        // IMPORTANT: create a new object to inform React Flow of changes
        return { ...node, data: { ...node.data, ...data } };
      }),
    });
  },

  setSelectedNodeId: (id) => set({ selectedNodeId: id }),
}));
```

### Connect Store to ReactFlow

```tsx
function Canvas() {
  const nodes = useWorkflowStore((s) => s.nodes);
  const edges = useWorkflowStore((s) => s.edges);
  const onNodesChange = useWorkflowStore((s) => s.onNodesChange);
  const onEdgesChange = useWorkflowStore((s) => s.onEdgesChange);
  const onConnect = useWorkflowStore((s) => s.onConnect);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      fitView
    >
      <Background />
      <Controls />
      <MiniMap />
    </ReactFlow>
  );
}
```

### Custom Node Accessing Store

```tsx
const WorkflowNode = memo(function WorkflowNode({ id, data }: NodeProps) {
  // Surgical selector -- only re-renders when THIS node's color changes
  const updateNodeData = useWorkflowStore((s) => s.updateNodeData);

  return (
    <div className="workflow-node">
      <Handle type="target" position={Position.Left} />
      <span>{data.label}</span>
      <input
        className="nodrag"
        type="color"
        defaultValue={data.color}
        onChange={(e) => updateNodeData(id, { color: e.target.value })}
      />
      <Handle type="source" position={Position.Right} />
    </div>
  );
});
```

### Undo/Redo with Zustand

#### Option A: Zundo middleware (recommended)

```bash
npm install zundo
```

```tsx
import { create } from 'zustand';
import { temporal } from 'zundo';

const useWorkflowStore = create<WorkflowState>()(
  temporal(
    (set, get) => ({
      nodes: [],
      edges: [],
      // ... same store as above
    }),
    {
      // Only track meaningful changes (skip drag position updates)
      equality: (pastState, currentState) =>
        JSON.stringify(pastState.nodes.map(n => ({ ...n, position: undefined }))) ===
        JSON.stringify(currentState.nodes.map(n => ({ ...n, position: undefined }))) &&
        JSON.stringify(pastState.edges) === JSON.stringify(currentState.edges),
    },
  ),
);

// Usage:
const { undo, redo, pastStates, futureStates } = useWorkflowStore.temporal.getState();
```

#### Option B: Manual snapshot stack

```tsx
type HistoryState = {
  past: Array<{ nodes: Node[]; edges: Edge[] }>;
  future: Array<{ nodes: Node[]; edges: Edge[] }>;
};

const useHistoryStore = create<HistoryState & {
  takeSnapshot: () => void;
  undo: () => void;
  redo: () => void;
}>((set, get) => ({
  past: [],
  future: [],

  takeSnapshot: () => {
    const { nodes, edges } = useWorkflowStore.getState();
    set({
      past: [...get().past, { nodes: structuredClone(nodes), edges: structuredClone(edges) }],
      future: [],
    });
  },

  undo: () => {
    const { past } = get();
    if (past.length === 0) return;

    const { nodes, edges } = useWorkflowStore.getState();
    const previous = past[past.length - 1];

    set({
      past: past.slice(0, -1),
      future: [{ nodes, edges }, ...get().future],
    });

    useWorkflowStore.getState().setNodes(previous.nodes);
    useWorkflowStore.getState().setEdges(previous.edges);
  },

  redo: () => {
    const { future } = get();
    if (future.length === 0) return;

    const { nodes, edges } = useWorkflowStore.getState();
    const next = future[0];

    set({
      past: [...get().past, { nodes, edges }],
      future: future.slice(1),
    });

    useWorkflowStore.getState().setNodes(next.nodes);
    useWorkflowStore.getState().setEdges(next.edges);
  },
}));
```

### Syncing with Backend

```tsx
import { debounce } from 'lodash-es';

// Subscribe to store changes and sync
const unsubscribe = useWorkflowStore.subscribe(
  (state) => ({ nodes: state.nodes, edges: state.edges }),
  debounce(async ({ nodes, edges }) => {
    await api.saveWorkflow(workflowId, {
      nodes: nodes.map(serializeNode),
      edges: edges.map(serializeEdge),
    });
  }, 2000),
  { equalityFn: (a, b) => JSON.stringify(a) === JSON.stringify(b) },
);
```

---

## 11. Validation

### Prevent Self-Connections + Type Checking

```tsx
type HandleType = 'string' | 'number' | 'object' | 'array' | 'any';

const isValidConnection = useCallback(
  (connection: Connection) => {
    const nodes = getNodes();
    const edges = getEdges();

    // Prevent self-connections
    if (connection.source === connection.target) return false;

    // Prevent duplicate edges
    const isDuplicate = edges.some(
      (e) =>
        e.source === connection.source &&
        e.target === connection.target &&
        e.sourceHandle === connection.sourceHandle &&
        e.targetHandle === connection.targetHandle,
    );
    if (isDuplicate) return false;

    // Type-safe handles
    const sourceNode = nodes.find((n) => n.id === connection.source);
    const targetNode = nodes.find((n) => n.id === connection.target);
    if (!sourceNode || !targetNode) return false;

    const sourceHandle = sourceNode.data.outputs?.find(
      (h: { id: string; type: HandleType }) => h.id === connection.sourceHandle,
    );
    const targetHandle = targetNode.data.inputs?.find(
      (h: { id: string; type: HandleType }) => h.id === connection.targetHandle,
    );

    if (!sourceHandle || !targetHandle) return false;

    // Type compatibility check
    if (targetHandle.type === 'any' || sourceHandle.type === 'any') return true;
    return sourceHandle.type === targetHandle.type;
  },
  [getNodes, getEdges],
);
```

### Cycle Detection (DAG Enforcement)

```tsx
import { getOutgoers, type Node, type Edge, type Connection } from '@xyflow/react';

const isValidConnection = useCallback(
  (connection: Connection) => {
    const nodes = getNodes();
    const edges = getEdges();
    const target = nodes.find((n) => n.id === connection.target);

    if (!target) return false;
    if (connection.source === connection.target) return false;

    // DFS cycle detection
    const hasCycle = (node: Node, visited = new Set<string>()): boolean => {
      if (visited.has(node.id)) return false;
      visited.add(node.id);

      for (const outgoer of getOutgoers(node, nodes, edges)) {
        if (outgoer.id === connection.source) return true;
        if (hasCycle(outgoer, visited)) return true;
      }
      return false;
    };

    return !hasCycle(target);
  },
  [getNodes, getEdges],
);
```

### Visual Feedback During Connection

Handles get `connecting` and `valid` CSS classes during drag-to-connect:

```css
/* Highlight valid drop targets */
.react-flow__handle.connecting {
  box-shadow: 0 0 0 3px rgba(137, 180, 250, 0.4);
}

.react-flow__handle.valid {
  background: #a6e3a1;
  box-shadow: 0 0 0 3px rgba(166, 227, 161, 0.4);
}

/* Invalid connections */
.react-flow__handle.connecting:not(.valid) {
  background: #f38ba8;
}
```

### Connection Events

```tsx
<ReactFlow
  isValidConnection={isValidConnection}
  onConnectStart={(_, { nodeId, handleType }) => {
    // Highlight compatible handles
    highlightCompatibleHandles(nodeId, handleType);
  }}
  onConnectEnd={() => {
    // Clear highlights
    clearHandleHighlights();
  }}
  onConnect={(connection) => {
    // Take undo snapshot before adding edge
    takeSnapshot();
  }}
/>
```

---

## 12. Minimap, Controls, Background

### MiniMap

```tsx
import { MiniMap } from '@xyflow/react';

<MiniMap
  position="bottom-right"
  pannable={true}
  zoomable={true}
  zoomStep={10}
  maskColor="rgba(0, 0, 0, 0.6)"
  bgColor="#11111b"
  nodeColor={(node) => {
    switch (node.type) {
      case 'llmCall': return '#89b4fa';
      case 'condition': return '#f9e2af';
      case 'trigger': return '#a6e3a1';
      default: return '#585b70';
    }
  }}
  nodeStrokeColor="transparent"
  nodeBorderRadius={4}
/>
```

### Custom MiniMap Node (SVG only)

```tsx
function MiniMapNode({ x, y, width, height, color }: MiniMapNodeProps) {
  return (
    <rect
      x={x}
      y={y}
      width={width}
      height={height}
      rx={4}
      fill={color}
      stroke="none"
      opacity={0.8}
    />
  );
}

<MiniMap nodeComponent={MiniMapNode} />
```

### Controls

```tsx
import { Controls, ControlButton } from '@xyflow/react';

<Controls
  position="bottom-left"
  orientation="vertical"
  showZoom={true}
  showFitView={true}
  showInteractive={true}
  fitViewOptions={{ padding: 0.2 }}
  onFitView={() => console.log('Fit view triggered')}
>
  {/* Custom buttons */}
  <ControlButton onClick={runLayout} title="Auto Layout">
    <LayoutIcon />
  </ControlButton>
  <ControlButton onClick={undo} title="Undo (Ctrl+Z)">
    <UndoIcon />
  </ControlButton>
  <ControlButton onClick={redo} title="Redo (Ctrl+Shift+Z)">
    <RedoIcon />
  </ControlButton>
</Controls>
```

### Background

```tsx
import { Background, BackgroundVariant } from '@xyflow/react';

// Dots (default)
<Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#313244" />

// Lines
<Background variant={BackgroundVariant.Lines} gap={40} color="rgba(255,255,255,0.03)" />

// Cross
<Background variant={BackgroundVariant.Cross} gap={30} size={2} />
```

### Panel

```tsx
import { Panel } from '@xyflow/react';

<Panel position="top-left">
  <div className="toolbar">
    <button onClick={runLayout}>Auto Layout</button>
    <select onChange={changeDirection}>
      <option value="TB">Top to Bottom</option>
      <option value="LR">Left to Right</option>
    </select>
  </div>
</Panel>

<Panel position="top-right">
  <ColorModeSwitcher />
</Panel>
```

### NodeToolbar

Shows a toolbar attached to a node when selected:

```tsx
import { NodeToolbar, Position } from '@xyflow/react';

function WorkflowNode({ id, data, selected }: NodeProps) {
  return (
    <>
      <NodeToolbar
        isVisible={selected}
        position={Position.Top}
        offset={10}
      >
        <button onClick={() => duplicateNode(id)}>Duplicate</button>
        <button onClick={() => deleteNode(id)}>Delete</button>
        <button onClick={() => openConfig(id)}>Configure</button>
      </NodeToolbar>

      <div className="workflow-node">
        <Handle type="target" position={Position.Left} />
        <span>{data.label}</span>
        <Handle type="source" position={Position.Right} />
      </div>
    </>
  );
}
```

### NodeResizer

Allow nodes to be resized (useful for group nodes):

```tsx
import { NodeResizer } from '@xyflow/react';

function ResizableGroupNode({ selected }: NodeProps) {
  return (
    <>
      <NodeResizer
        isVisible={selected}
        minWidth={200}
        minHeight={150}
        color="#89b4fa"
        handleStyle={{ width: 8, height: 8 }}
      />
      <div className="group-content">
        {/* Children rendered by React Flow via parentId */}
      </div>
    </>
  );
}
```

---

## 13. React Flow Pro / React Flow UI

### React Flow UI (Free -- shadcn CLI)

Install components via the shadcn CLI:

```bash
npx shadcn@latest add https://ui.reactflow.dev/animated-svg-edge
npx shadcn@latest add https://ui.reactflow.dev/data-edge
```

Available components include:
- **AnimatedSvgEdge** -- animate custom SVG shapes along edges
- **DataEdge** -- edges that display data labels
- **Database Schema Node** -- visualize tables and relationships
- **Zoom Slider** -- viewport zoom control

### React Flow Pro (Subscription)

Pro features available through subscription:
- **Workflow Editor Template** -- full Next.js template with Zustand, shadcn/ui, AI SDK
- **AI Workflow Editor Template** -- AI-specific node types, Vercel AI SDK integration
- **Dynamic Layouting Example** -- d3-hierarchy auto-layout with smooth transitions
- **Auto Layout Hook** -- reusable hook supporting dagre, d3-hierarchy, ELK
- **Selection Grouping** -- dynamic group creation from selected nodes
- **Copy and Paste** -- Ctrl+C/V for nodes and edges
- **Undo/Redo** -- snapshot-based history with keyboard shortcuts
- **Shapes** -- diamond, hexagon, circle node shapes with SVG
- Priority GitHub issues and 1:1 support from xyflow team

### Pro Template Architecture

```
Next.js App
  + @xyflow/react (canvas)
  + Zustand (state management)
  + shadcn/ui + Tailwind CSS (styling)
  + Vercel AI SDK (LLM integration -- AI template)
```

---

## 14. Common Pitfalls

### 1. Defining nodeTypes/edgeTypes Inside Components

```tsx
// BAD -- causes ALL nodes to remount on every render
function Canvas() {
  const nodeTypes = { custom: CustomNode }; // new object each render
  return <ReactFlow nodeTypes={nodeTypes} />;
}

// GOOD -- stable reference
const nodeTypes = { custom: CustomNode };
function Canvas() {
  return <ReactFlow nodeTypes={nodeTypes} />;
}
```

### 2. Subscribing to Full Node/Edge Arrays in Custom Nodes

Every custom node that reads `useStore((s) => s.nodes)` will re-render on every drag, zoom, and pan. Use surgical selectors:

```tsx
// BAD
const allNodes = useStore((s) => s.nodes);
const myNode = allNodes.find((n) => n.id === id);

// GOOD -- only re-renders when THIS node changes
const myData = useStore(
  useCallback((s) => s.nodes.find((n) => n.id === id)?.data, [id]),
);
```

### 3. Not Memoizing Props

All objects and functions passed to `<ReactFlow>` must be memoized. React Flow's documentation warns: **optimize early** -- unlike general React advice, performance issues in React Flow are hard to fix retroactively.

### 4. Using display: none on Handles

Breaks dimension calculation. Use `visibility: hidden` or `opacity: 0` instead.

### 5. Not Calling useUpdateNodeInternals

When dynamically adding/removing handles, React Flow does not automatically detect the change. You must call `useUpdateNodeInternals()` with the node ID.

### 6. Parent Nodes After Children in Array

Children nodes must come after their parent in the nodes array. React Flow processes them in order and will fail silently if a child references a parent that has not been processed yet.

### 7. Mutating Node/Edge Objects

Always create new objects when updating state:

```tsx
// BAD -- mutates existing object
node.data.label = 'New Label';

// GOOD -- new object reference
return { ...node, data: { ...node.data, label: 'New Label' } };
```

### 8. Not Wrapping with ReactFlowProvider

`useReactFlow()`, `useStore()`, and other hooks require the `<ReactFlowProvider>` ancestor. If your `<ReactFlow>` component and sidebar/panels are siblings, wrap the common parent.

### 9. Forgetting base.css

If you use `@xyflow/react/dist/base.css` instead of `style.css`, that is fine -- but you MUST import the base styles. They contain layout rules required for React Flow to function.

### 10. Animated Edges on Large Graphs

`animated: true` uses `stroke-dasharray` CSS which is CPU-intensive. With 100+ edges, frame rate drops noticeably. Use `animateMotion` SVG animation or remove animation during interactions.

### 11. Not Handling Edge Reconnection

v12 introduced `edgesReconnectable` and `onReconnect`. If you want users to re-route existing edges:

```tsx
<ReactFlow
  edgesReconnectable={true}
  onReconnect={(oldEdge, newConnection) => {
    setEdges((edges) =>
      edges.map((e) => {
        if (e.id === oldEdge.id) {
          return { ...e, ...newConnection };
        }
        return e;
      }),
    );
  }}
/>
```

### 12. Not Setting connectionMode for Loose Connections

By default, connections require source-to-target. For bidirectional flows:

```tsx
<ReactFlow connectionMode="loose" />
```

---

## 15. ReactFlow Component Props Reference

Key props for enterprise workflow builders:

### Viewport & Grid

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `fitView` | `boolean` | `false` | Auto-zoom to fit all nodes on mount |
| `minZoom` | `number` | `0.5` | Minimum zoom level |
| `maxZoom` | `number` | `2` | Maximum zoom level |
| `snapToGrid` | `boolean` | `false` | Snap nodes to grid |
| `snapGrid` | `[number, number]` | `[15, 15]` | Grid size |
| `onlyRenderVisibleElements` | `boolean` | `false` | Viewport culling |
| `colorMode` | `'light' \| 'dark' \| 'system'` | `'system'` | Theme mode |

### Connection & Validation

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `isValidConnection` | `(connection: Connection) => boolean` | - | Global connection validator |
| `connectionMode` | `'strict' \| 'loose'` | `'strict'` | Strict = source-to-target only |
| `connectionLineType` | `ConnectionLineType` | `'bezier'` | Visual style of in-progress connection |
| `connectionRadius` | `number` | `20` | Drop radius around handle |
| `connectOnClick` | `boolean` | `true` | Click handles to connect |
| `autoPanOnConnect` | `boolean` | `true` | Pan canvas during connect |
| `autoPanOnNodeDrag` | `boolean` | `true` | Pan canvas during drag |

### Edge Configuration

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `defaultEdgeOptions` | `DefaultEdgeOptions` | - | Defaults for all new edges |
| `edgesReconnectable` | `boolean` | `false` | Allow re-routing edges |
| `reconnectRadius` | `number` | `10` | Drop radius for reconnect |
| `elevateEdgesOnSelect` | `boolean` | `false` | Raise z-index on selection |

### Interaction

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `nodesDraggable` | `boolean` | `true` | Enable node dragging |
| `nodesConnectable` | `boolean` | `true` | Enable handle connections |
| `elementsSelectable` | `boolean` | `true` | Enable selection |
| `selectionOnDrag` | `boolean` | `false` | Drag to create selection box |
| `panOnDrag` | `boolean \| number[]` | `true` | Pan behavior / mouse buttons |
| `zoomOnScroll` | `boolean` | `true` | Scroll to zoom |
| `zoomOnPinch` | `boolean` | `true` | Pinch to zoom |
| `panOnScroll` | `boolean` | `false` | Scroll to pan |

### Keyboard

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `deleteKeyCode` | `KeyCode \| null` | `'Backspace'` | Delete selected elements |
| `selectionKeyCode` | `KeyCode \| null` | `'Shift'` | Selection box trigger |
| `multiSelectionKeyCode` | `KeyCode \| null` | `'Meta'` | Add to selection |
| `panActivationKeyCode` | `KeyCode \| null` | `'Space'` | Activate pan mode |
| `zoomActivationKeyCode` | `KeyCode \| null` | `'Meta'` | Activate zoom mode |

### Event Handlers

| Prop | Signature |
|------|-----------|
| `onConnect` | `(connection: Connection) => void` |
| `onConnectStart` | `(event, params: { nodeId, handleType }) => void` |
| `onConnectEnd` | `(event) => void` |
| `onReconnect` | `(oldEdge: Edge, newConnection: Connection) => void` |
| `onNodesChange` | `(changes: NodeChange[]) => void` |
| `onEdgesChange` | `(changes: EdgeChange[]) => void` |
| `onNodeDrag` | `(event, node: Node, nodes: Node[]) => void` |
| `onNodeDragStart` | `(event, node: Node, nodes: Node[]) => void` |
| `onNodeDragStop` | `(event, node: Node, nodes: Node[]) => void` |
| `onNodeClick` | `(event, node: Node) => void` |
| `onNodeDoubleClick` | `(event, node: Node) => void` |
| `onNodeContextMenu` | `(event, node: Node) => void` |
| `onSelectionChange` | `(params: { nodes: Node[], edges: Edge[] }) => void` |
| `onInit` | `(instance: ReactFlowInstance) => void` |
| `onError` | `(id: string, message: string) => void` |

---

## Quick-Start Assembly

A minimal but enterprise-structured setup:

```tsx
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  Panel,
  ConnectionLineType,
  MarkerType,
  type ColorMode,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

const nodeTypes = {
  workflow: WorkflowNode,
  llmCall: LLMCallNode,
  condition: ConditionNode,
  trigger: TriggerNode,
  group: ResizableGroupNode,
} satisfies NodeTypes;

const edgeTypes = {
  animated: AnimatedSVGEdge,
  button: ButtonEdge,
} satisfies EdgeTypes;

const defaultEdgeOptions = {
  type: 'smoothstep',
  markerEnd: { type: MarkerType.ArrowClosed },
  animated: false,
};

const snapGrid: [number, number] = [20, 20];

function WorkflowCanvas() {
  const nodes = useWorkflowStore((s) => s.nodes);
  const edges = useWorkflowStore((s) => s.edges);
  const onNodesChange = useWorkflowStore((s) => s.onNodesChange);
  const onEdgesChange = useWorkflowStore((s) => s.onEdgesChange);
  const onConnect = useWorkflowStore((s) => s.onConnect);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      defaultEdgeOptions={defaultEdgeOptions}
      connectionLineType={ConnectionLineType.SmoothStep}
      isValidConnection={isValidConnection}
      snapToGrid={true}
      snapGrid={snapGrid}
      colorMode="dark"
      fitView
      minZoom={0.1}
      maxZoom={4}
      deleteKeyCode="Backspace"
      multiSelectionKeyCode="Meta"
      selectionOnDrag={false}
      connectOnClick={true}
      edgesReconnectable={true}
    >
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
      <Controls position="bottom-left" />
      <MiniMap
        position="bottom-right"
        pannable
        zoomable
        nodeColor={miniMapNodeColor}
      />
      <Panel position="top-right">
        <WorkflowToolbar />
      </Panel>
    </ReactFlow>
  );
}

function App() {
  return (
    <ReactFlowProvider>
      <DnDProvider>
        <div className="app-layout">
          <Sidebar />
          <WorkflowCanvas />
          <ConfigPanel />
        </div>
      </DnDProvider>
    </ReactFlowProvider>
  );
}
```

---

## Sources

- [Custom Nodes](https://reactflow.dev/learn/customization/custom-nodes)
- [Handles](https://reactflow.dev/learn/customization/handles)
- [Handle API Reference](https://reactflow.dev/api-reference/components/handle)
- [Theming](https://reactflow.dev/learn/customization/theming)
- [Dark Mode Example](https://reactflow.dev/examples/styling/dark-mode)
- [Animating Edges](https://reactflow.dev/examples/edges/animating-edges)
- [Animated SVG Edge Component](https://reactflow.dev/ui/components/animated-svg-edge)
- [Custom Edges](https://reactflow.dev/examples/edges/custom-edges)
- [Drag and Drop](https://reactflow.dev/examples/interaction/drag-and-drop)
- [Dagre Layout](https://reactflow.dev/examples/layout/dagre)
- [ELK.js Layout](https://reactflow.dev/examples/layout/elkjs)
- [ELK.js Multiple Handles](https://reactflow.dev/examples/layout/elkjs-multiple-handles)
- [Layouting Overview](https://reactflow.dev/learn/layouting/layouting)
- [Auto Layout (Pro)](https://reactflow.dev/examples/layout/auto-layout)
- [Performance](https://reactflow.dev/learn/advanced-use/performance)
- [Validation](https://reactflow.dev/examples/interaction/validation)
- [Preventing Cycles](https://reactflow.dev/examples/interaction/prevent-cycles)
- [Connection Limit](https://reactflow.dev/examples/nodes/connection-limit)
- [Context Menu](https://reactflow.dev/examples/interaction/context-menu)
- [Sub Flows](https://reactflow.dev/examples/grouping/sub-flows)
- [Sub Flows Layouting](https://reactflow.dev/learn/layouting/sub-flows)
- [State Management with Zustand](https://reactflow.dev/learn/advanced-use/state-management)
- [Undo and Redo (Pro)](https://reactflow.dev/examples/interaction/undo-redo)
- [MiniMap API](https://reactflow.dev/api-reference/components/minimap)
- [Controls API](https://reactflow.dev/api-reference/components/controls)
- [ReactFlow Component API](https://reactflow.dev/api-reference/react-flow)
- [React Flow UI](https://reactflow.dev/ui)
- [AI Workflow Editor Template](https://reactflow.dev/ui/templates/ai-workflow-editor)
- [Workflow Editor Template](https://reactflow.dev/ui/templates/workflow-editor)
- [Shapes (Pro)](https://reactflow.dev/examples/nodes/shapes)
- [Selection Grouping (Pro)](https://reactflow.dev/examples/grouping/selection-grouping)
- [Copy and Paste (Pro)](https://reactflow.dev/examples/interaction/copy-paste)
- [React Flow v12 Migration](https://reactflow.dev/learn/troubleshooting/migrate-to-v12)
- [React Flow v12.5.0 Release](https://reactflow.dev/whats-new/2025-03-27)
- [Tuning Edge Animations Performance](https://liambx.com/blog/tuning-edge-animations-reactflow-optimal-performance)
- [Synergy Codes Performance Guide](https://www.synergycodes.com/webbook/guide-to-optimize-react-flow-project-performance)
- [Zundo (Zustand Undo Middleware)](https://github.com/charkour/zundo)
