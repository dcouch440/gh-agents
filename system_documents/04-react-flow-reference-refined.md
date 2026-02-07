# React Flow v12+ Reference — Enterprise AI Workflow Builder

`@xyflow/react` v12.10.0 · TypeScript · React 19 · MUI + Framer Motion available

**Note:** React Flow recommends Zustand for state. Our CLAUDE.md says "no external state libraries." Decide per-feature — the workflow canvas may warrant an exception. Patterns below show both Zustand and vanilla React approaches.

---

## 1. Custom Nodes

Custom nodes = React components. React Flow injects `id`, `data`, `position`, `selected`, `dragging` via `NodeProps<T>`.

```tsx
type WorkflowNodeData = {
  label: string;
  status: 'idle' | 'running' | 'success' | 'error';
  inputs: Array<{ id: string; label: string; type: HandleType }>;
  outputs: Array<{ id: string; label: string; type: HandleType }>;
};

type WorkflowNode = Node<WorkflowNodeData, 'workflow'>;

function WorkflowNode({ id, data, selected }: NodeProps<WorkflowNode>) {
  return (
    <div className={`workflow-node ${data.status} ${selected ? 'selected' : ''}`}>
      {data.inputs.map((input, i) => (
        <Handle key={input.id} type="target" position={Position.Left} id={input.id}
          style={{ top: `${((i + 1) / (data.inputs.length + 1)) * 100}%` }} />
      ))}
      <div className="workflow-node__header">
        <span>{data.label}</span>
      </div>
      {data.outputs.map((output, i) => (
        <Handle key={output.id} type="source" position={Position.Right} id={output.id}
          style={{ top: `${((i + 1) / (data.outputs.length + 1)) * 100}%` }} />
      ))}
    </div>
  );
}
```

### Critical Rules

- **Define `nodeTypes` / `edgeTypes` OUTSIDE component body** — inside = all nodes remount every render.
- **`nodrag` class** on interactive elements (inputs, selects, textareas) to prevent drag.
- **`nowheel` class** on scrollable elements to prevent canvas zoom.
- **`memo()` all custom nodes** — React Flow docs say "optimize early."
- **v12 `width`/`height`** on node objects enables layout without DOM measurement.

### Diamond / Custom Shapes

```tsx
function DiamondNode({ data }: NodeProps) {
  return (
    <div className="diamond-node" style={{ width: 100, height: 100 }}>
      <svg viewBox="0 0 100 100">
        <polygon points="50,0 100,50 50,100 0,50" fill="var(--node-bg)" stroke="var(--node-border)" />
      </svg>
      <Handle type="target" position={Position.Top} style={{ left: '50%', top: '-4px' }} />
      <Handle type="source" position={Position.Bottom} style={{ left: '50%', bottom: '-4px' }} />
    </div>
  );
}
```

---

## 2. Handles (Ports)

### Positioning: Inside / Edge / Outside

```css
/* Default: sits ON the node edge */
.react-flow__handle { width: 12px; height: 12px; border-radius: 50%; }

/* INSIDE the node — pull inward */
.react-flow__handle.inside { transform: translate(4px, 0); }

/* OUTSIDE the node — push outward */
.react-flow__handle.outside { transform: translate(-6px, 0); }
```

### Custom Handle Visual (Children API)

Wrap any element as a handle by hiding the default and using pointer-events:

```tsx
<Handle type={type} position={position} id={id}
  style={{ background: 'none', border: 'none', width: 16, height: 16 }}>
  <div style={{
    width: 12, height: 12,
    borderRadius: handleType === 'string' ? '50%' : 2,
    background: color, pointerEvents: 'none',
    position: 'absolute', top: '50%', left: '50%',
    transform: 'translate(-50%, -50%)',
  }} />
</Handle>
```

### Dynamic Handles

When adding/removing handles programmatically, you MUST call `useUpdateNodeInternals()`:

```tsx
const updateNodeInternals = useUpdateNodeInternals();
// After mutating handles:
updateNodeInternals(nodeId);
```

### Connection Limit Per Handle

```tsx
function LimitedHandle({ connectionCount, ...props }: HandleProps & { connectionCount: number }) {
  const connections = useNodeConnections({ handleType: props.type });
  return <Handle {...props} isConnectable={connections.length < connectionCount} />;
}
```

### Connection Feedback CSS

```css
.react-flow__handle.connecting { box-shadow: 0 0 0 3px rgba(137, 180, 250, 0.4); }
.react-flow__handle.valid { background: #a6e3a1; }
.react-flow__handle.connecting:not(.valid) { background: #f38ba8; }
```

**Never `display: none` on handles** — breaks dimension calc. Use `visibility: hidden` or `opacity: 0`.

---

## 3. Drag and Drop (Sidebar → Canvas)

Uses Pointer Events API for mouse + touch. Three pieces: context provider, sidebar palette, canvas drop handler.

### DnD Hook (Condensed)

```tsx
const useDnD = () => {
  const { screenToFlowPosition } = useReactFlow();
  // ... context state: isDragging, dropAction

  const onDragStart = (event: React.PointerEvent, onDrop: (pos: XYPosition) => void) => {
    event.preventDefault();
    (event.target as HTMLElement).setPointerCapture(event.pointerId);
    setIsDragging(true);
    setDropAction(onDrop);
  };

  // On pointerup: check if drop target is inside .react-flow
  // Convert screen coords → flow coords via screenToFlowPosition({ x: event.clientX, y: event.clientY })
  // Call dropAction with flow position
};
```

### Sidebar Item

```tsx
<div onPointerDown={(e) => onDragStart(e, (position) => {
  addNode({ id: crypto.randomUUID(), type: nodeType, position, data: { label: `New ${nodeType}` } });
})}>
  {template.label}
</div>
```

### Ghost Preview

Fixed-position div following pointer with `pointerEvents: 'none'`, `zIndex: 9999`.

### Snap to Grid

```tsx
<ReactFlow snapToGrid={true} snapGrid={[20, 20]} />
```

### Assembly

```tsx
<ReactFlowProvider>
  <DnDProvider>
    <Sidebar />
    <Canvas />
  </DnDProvider>
</ReactFlowProvider>
```

---

## 4. Styling & Theming

### CSS Variables (Override under `.react-flow`)

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
  /* Selection */
  --xy-selection-background-color-default: rgba(0, 89, 220, 0.08);
  --xy-selection-border-default: 1px dotted rgba(0, 89, 220, 0.8);
  /* Background */
  --xy-background-pattern-dots-color-default: #91919a;
  /* Resize */
  --xy-resize-background-color-default: #3367d9;
}
```

### Dark Mode

Built-in: `<ReactFlow colorMode="dark" />` adds `.dark` class. Override:

```css
.dark .react-flow {
  --xy-node-background-color-default: #1e1e2e;
  --xy-node-border-default: 1px solid rgba(255, 255, 255, 0.08);
  --xy-node-color-default: #cdd6f4;
  --xy-node-boxshadow-selected-default: 0 0 0 2px #89b4fa;
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

### Glassmorphism Node

```css
.glass-node {
  background: rgba(30, 30, 46, 0.6);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2), inset 0 1px 0 rgba(255, 255, 255, 0.05);
  padding: 16px;
  color: #cdd6f4;
  transition: box-shadow 0.2s ease, border-color 0.2s ease;
}
.glass-node:hover {
  border-color: rgba(137, 180, 250, 0.3);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3), 0 0 0 1px rgba(137, 180, 250, 0.1);
}
.glass-node.selected {
  border-color: #89b4fa;
  box-shadow: 0 0 0 2px rgba(137, 180, 250, 0.3), 0 4px 24px rgba(0, 0, 0, 0.3);
}
```

### Import Order

```tsx
import '@xyflow/react/dist/style.css'; // Full styles (or base.css for minimal)
// Then your own CSS/MUI theme
```

---

## 5. Edge Animations

### Approach A: animateMotion (Best Performance)

Animate shapes traveling along the edge path. **Much faster than stroke-dasharray.**

```tsx
function AnimatedEdge({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }: EdgeProps) {
  const [edgePath] = getSmoothStepPath({ sourceX, sourceY, sourcePosition, targetX, targetY, targetPosition });
  return (
    <>
      <BaseEdge id={id} path={edgePath} />
      <circle r="4" fill="#89b4fa">
        <animateMotion dur="2s" repeatCount="indefinite" path={edgePath} />
      </circle>
    </>
  );
}
```

### Approach B: Multiple Particles

```tsx
{[0, 0.33, 0.66].map((delay, i) => (
  <circle key={i} r="3" fill="#a6e3a1" opacity="0.8">
    <animateMotion dur="3s" repeatCount="indefinite" path={edgePath} begin={`${delay * 3}s`} />
  </circle>
))}
```

### Approach C: Gradient Pulse

```tsx
<defs>
  <linearGradient id={`grad-${id}`}>
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
<BaseEdge id={id} path={edgePath} style={{ stroke: `url(#grad-${id})`, strokeWidth: 2 }} />
```

### Performance Comparison (100 edges)

| Technique | Frame Drop |
|-----------|-----------|
| `animated: true` (stroke-dasharray) | ~5 frames |
| `animateMotion` | ~2-3 frames |
| No animation | ~0-1 frames |

**Remove animations during drag for max responsiveness.**

---

## 6. Edge Types & Custom Edges

Built-in: `default` (bezier), `straight`, `step`, `smoothstep` (recommended for workflows).

Path helpers: `getBezierPath()`, `getSmoothStepPath()`, `getStraightPath()` — all return `[edgePath, labelX, labelY]`.

### Edge with Delete Button

```tsx
function ButtonEdge({ id, ...props }: EdgeProps) {
  const { setEdges } = useReactFlow();
  const [edgePath, labelX, labelY] = getBezierPath(props);
  return (
    <>
      <BaseEdge id={id} path={edgePath} />
      <EdgeLabelRenderer>
        <div className="nodrag nopan" style={{
          position: 'absolute',
          transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
          pointerEvents: 'all',
        }}>
          <button onClick={() => setEdges((eds) => eds.filter((e) => e.id !== id))}>x</button>
        </div>
      </EdgeLabelRenderer>
    </>
  );
}
```

### Arrow Markers

```tsx
const defaultEdgeOptions = {
  type: 'smoothstep',
  markerEnd: { type: MarkerType.ArrowClosed, color: '#89b4fa' },
};
```

### Register (OUTSIDE component)

```tsx
const edgeTypes = { animated: AnimatedEdge, button: ButtonEdge } satisfies EdgeTypes;
```

---

## 7. Layout Algorithms

React Flow has NO built-in layout. Use external libs:

| Library | Best For | Trade-off |
|---------|----------|-----------|
| `@dagrejs/dagre` | Simple DAGs, fast | Deprecated, no async, fixed sizes |
| `elkjs` | Complex graphs, port routing, async | Heavier, complex API |
| `d3-hierarchy` | Strict trees | Single-root only |

### Dagre (Start Here)

```tsx
import dagre from '@dagrejs/dagre';

const getLayoutedElements = (nodes: Node[], edges: Edge[], direction = 'TB') => {
  const g = new dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: direction, nodesep: 50, ranksep: 80 });
  nodes.forEach((n) => g.setNode(n.id, { width: n.width ?? 250, height: n.height ?? 80 }));
  edges.forEach((e) => g.setEdge(e.source, e.target));
  dagre.layout(g);

  const isHorizontal = direction === 'LR';
  return {
    nodes: nodes.map((n) => {
      const pos = g.node(n.id);
      return {
        ...n,
        position: { x: pos.x - (n.width ?? 250) / 2, y: pos.y - (n.height ?? 80) / 2 },
        targetPosition: isHorizontal ? Position.Left : Position.Top,
        sourcePosition: isHorizontal ? Position.Right : Position.Bottom,
      };
    }),
    edges,
  };
};
```

### ELK (Escalate When Needed)

```tsx
import ELK from 'elkjs/lib/elk.bundled.js';
const elk = new ELK();

const getLayoutedElements = async (nodes: Node[], edges: Edge[]) => {
  const graph = {
    id: 'root',
    layoutOptions: { 'elk.algorithm': 'layered', 'elk.spacing.nodeNode': '80' },
    children: nodes.map((n) => ({ ...n, width: n.width ?? 250, height: n.height ?? 80 })),
    edges: edges.map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  };
  const result = await elk.layout(graph);
  return {
    nodes: (result.children ?? []).map((n) => ({ ...n, position: { x: n.x!, y: n.y! } })),
    edges,
  };
};
```

### ELK Port Constraints (Multiple Handles)

```tsx
ports: [
  ...inputs.map((p) => ({ id: p.id, properties: { side: 'WEST' } })),
  ...outputs.map((p) => ({ id: p.id, properties: { side: 'EAST' } })),
],
properties: { 'org.eclipse.elk.portConstraints': 'FIXED_ORDER' },
```

### Auto-Layout Hook

```tsx
const useAutoLayout = () => {
  const { getNodes, getEdges, setNodes, setEdges, fitView } = useReactFlow();
  const nodesInitialized = useNodesInitialized();

  const runLayout = useCallback(async () => {
    const { nodes, edges } = await getLayoutedElements(getNodes(), getEdges());
    setNodes(nodes); setEdges(edges);
    requestAnimationFrame(() => fitView({ padding: 0.2 }));
  }, [getNodes, getEdges, setNodes, setEdges, fitView]);

  useEffect(() => { if (nodesInitialized) runLayout(); }, [nodesInitialized]);
  return { runLayout };
};
```

**Tips:** Use `useNodesInitialized` to wait for DOM measurement before layout. Don't relayout on every change — provide a button. Dagre breaks with sub-flows; use ELK for nested graphs.

---

## 8. Performance

### Must-Do

- **`memo()` all custom nodes/edges**
- **`nodeTypes`/`edgeTypes` defined OUTSIDE component** (stable reference)
- **Memoize ALL callback/object props** passed to `<ReactFlow>`
- **Surgical store selectors** — never `useStore((s) => s.nodes)` in a custom node

```tsx
// BAD — re-renders on every drag
const allNodes = useStore((s) => s.nodes);
// GOOD — only re-renders when THIS node's data changes
const myData = useStore(useCallback((s) => s.nodes.find((n) => n.id === id)?.data, [id]));
```

### Large Graphs (1000+)

- `onlyRenderVisibleElements={true}` (viewport culling)
- Avoid `animated: true` edges — use `animateMotion` or skip animation
- Collapse subtrees via `node.hidden = true`
- Debounce layout recalculations
- Disable expensive CSS during drag:

```css
.react-flow__node.dragging .glass-node {
  backdrop-filter: none;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  transition: none;
}
```

---

## 9. Interactivity

### Multi-Select & Keys

```tsx
<ReactFlow
  selectionKeyCode="Shift"        // Shift+drag for selection box
  multiSelectionKeyCode="Meta"    // Cmd+click to toggle
  deleteKeyCode="Backspace"
  panActivationKeyCode="Space"
  selectionOnDrag={false}         // true = drag-select without key (use panOnDrag={[1,2]} for pan)
/>
```

### Context Menu

```tsx
const onNodeContextMenu = useCallback((event: React.MouseEvent, node: Node) => {
  event.preventDefault();
  const pane = ref.current!.getBoundingClientRect();
  setMenu({
    id: node.id,
    x: Math.min(event.clientX - pane.left, pane.width - 200),
    y: Math.min(event.clientY - pane.top, pane.height - 200),
  });
}, []);
// Close on pane click: onPaneClick={() => setMenu(null)}
```

### Node Grouping / Sub-Flows

```tsx
// Parent node
{ id: 'group-a', type: 'group', position: { x: 100, y: 100 }, style: { width: 400, height: 300 } }
// Child — relative position, constrained to parent
{ id: 'step-1', type: 'workflow', position: { x: 20, y: 50 }, parentId: 'group-a', extent: 'parent' }
```

**Parent nodes MUST appear before children in the array.**

### Collapsible Groups

Toggle `hidden` on children by `parentId`. Use `NodeResizer` for resizable groups.

### NodeToolbar (Attached to Selected Node)

```tsx
<NodeToolbar isVisible={selected} position={Position.Top} offset={10}>
  <button onClick={() => duplicateNode(id)}>Duplicate</button>
  <button onClick={() => deleteNode(id)}>Delete</button>
</NodeToolbar>
```

### Keyboard Shortcuts

```tsx
const undoPressed = useKeyPress(['Meta+z', 'Control+z']);
// Or custom: document.addEventListener('keydown', handler)
```

---

## 10. Validation

### Combined: Self-Connection + Duplicate + Type Check + Cycle Detection

```tsx
const isValidConnection = useCallback((connection: Connection) => {
  const nodes = getNodes();
  const edges = getEdges();

  // No self-connections
  if (connection.source === connection.target) return false;

  // No duplicate edges
  if (edges.some((e) =>
    e.source === connection.source && e.target === connection.target &&
    e.sourceHandle === connection.sourceHandle && e.targetHandle === connection.targetHandle
  )) return false;

  // Type-safe handles
  const sourceNode = nodes.find((n) => n.id === connection.source);
  const targetNode = nodes.find((n) => n.id === connection.target);
  if (!sourceNode || !targetNode) return false;

  const srcHandle = sourceNode.data.outputs?.find((h: Handle) => h.id === connection.sourceHandle);
  const tgtHandle = targetNode.data.inputs?.find((h: Handle) => h.id === connection.targetHandle);
  if (!srcHandle || !tgtHandle) return false;
  if (tgtHandle.type !== 'any' && srcHandle.type !== 'any' && srcHandle.type !== tgtHandle.type) return false;

  // DAG cycle detection (DFS from target)
  const target = targetNode;
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
}, [getNodes, getEdges]);
```

### Connection Events

```tsx
<ReactFlow
  isValidConnection={isValidConnection}
  onConnectStart={(_, { nodeId, handleType }) => highlightCompatibleHandles(nodeId, handleType)}
  onConnectEnd={() => clearHandleHighlights()}
  onConnect={(connection) => takeSnapshot()} // undo snapshot
/>
```

---

## 11. Built-In Components

### MiniMap

```tsx
<MiniMap position="bottom-right" pannable zoomable
  maskColor="rgba(0,0,0,0.6)" bgColor="#11111b"
  nodeColor={(node) => {
    switch (node.type) {
      case 'llmCall': return '#89b4fa';
      case 'condition': return '#f9e2af';
      default: return '#585b70';
    }
  }}
/>
```

Custom SVG node: `<MiniMap nodeComponent={MyMiniMapNode} />` (must use SVG elements only).

### Controls

```tsx
<Controls position="bottom-left" orientation="vertical">
  <ControlButton onClick={runLayout} title="Auto Layout"><LayoutIcon /></ControlButton>
</Controls>
```

### Background

```tsx
<Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#313244" />
```

Variants: `Dots` | `Lines` | `Cross`.

### Panel

```tsx
<Panel position="top-right"><WorkflowToolbar /></Panel>
```

### NodeResizer

```tsx
<NodeResizer isVisible={selected} minWidth={200} minHeight={150} color="#89b4fa" />
```

---

## 12. Common Pitfalls

1. **nodeTypes/edgeTypes inside component** — remounts ALL nodes every render
2. **`useStore((s) => s.nodes)` in custom node** — re-renders on every interaction
3. **Not memoizing props** — React Flow says "optimize early," unlike general React
4. **`display: none` on handles** — breaks dimension calc, use `opacity: 0`
5. **Not calling `useUpdateNodeInternals()`** after adding/removing handles dynamically
6. **Parent nodes after children in array** — fails silently
7. **Mutating node/edge objects** — always spread to create new references
8. **Missing `ReactFlowProvider`** — all hooks require it as ancestor
9. **Forgetting CSS import** — `@xyflow/react/dist/style.css` (or `base.css` minimum)
10. **`animated: true` on 100+ edges** — use `animateMotion` instead
11. **Missing `edgesReconnectable` + `onReconnect`** — v12 feature for re-routing edges
12. **`connectionMode="strict"` default** — set to `"loose"` for bidirectional flows

---

## 13. ReactFlow Props Quick Reference

### Viewport & Grid

| Prop | Default | Note |
|------|---------|------|
| `fitView` | `false` | Auto-fit on mount |
| `minZoom` / `maxZoom` | `0.5` / `2` | |
| `snapToGrid` / `snapGrid` | `false` / `[15,15]` | |
| `onlyRenderVisibleElements` | `false` | Viewport culling |
| `colorMode` | `'system'` | `'light' \| 'dark' \| 'system'` |

### Connections

| Prop | Default | Note |
|------|---------|------|
| `isValidConnection` | - | Global validator |
| `connectionMode` | `'strict'` | `'loose'` for bidirectional |
| `connectionLineType` | `'bezier'` | `SmoothStep` recommended |
| `connectionRadius` | `20` | Handle drop radius |
| `connectOnClick` | `true` | Click-to-connect |
| `edgesReconnectable` | `false` | Enable edge re-routing |

### Interaction

| Prop | Default | Note |
|------|---------|------|
| `selectionOnDrag` | `false` | Drag = selection box |
| `panOnDrag` | `true` | `[1,2]` for specific mouse buttons |
| `deleteKeyCode` | `'Backspace'` | `null` to disable |
| `selectionKeyCode` | `'Shift'` | |
| `multiSelectionKeyCode` | `'Meta'` | |

### Key Event Handlers

`onConnect`, `onConnectStart`, `onConnectEnd`, `onReconnect`, `onNodesChange`, `onEdgesChange`, `onNodeDrag`, `onNodeDragStart`, `onNodeDragStop`, `onNodeClick`, `onNodeDoubleClick`, `onNodeContextMenu`, `onSelectionChange`, `onInit`, `onError`

---

## 14. Quick-Start Assembly

```tsx
import {
  ReactFlow, ReactFlowProvider, Background, BackgroundVariant,
  Controls, MiniMap, Panel, ConnectionLineType, MarkerType,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

// OUTSIDE component
const nodeTypes = { workflow: WorkflowNode, condition: ConditionNode, group: GroupNode } satisfies NodeTypes;
const edgeTypes = { animated: AnimatedEdge, button: ButtonEdge } satisfies EdgeTypes;
const defaultEdgeOptions = { type: 'smoothstep', markerEnd: { type: MarkerType.ArrowClosed } };
const snapGrid: [number, number] = [20, 20];

function WorkflowCanvas() {
  // State via useReducer, Zustand, or useNodesState/useEdgesState
  return (
    <ReactFlow
      nodes={nodes} edges={edges}
      onNodesChange={onNodesChange} onEdgesChange={onEdgesChange} onConnect={onConnect}
      nodeTypes={nodeTypes} edgeTypes={edgeTypes} defaultEdgeOptions={defaultEdgeOptions}
      connectionLineType={ConnectionLineType.SmoothStep}
      isValidConnection={isValidConnection}
      snapToGrid snapGrid={snapGrid}
      colorMode="dark" fitView minZoom={0.1} maxZoom={4}
      edgesReconnectable connectOnClick
    >
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
      <Controls position="bottom-left" />
      <MiniMap position="bottom-right" pannable zoomable nodeColor={miniMapNodeColor} />
      <Panel position="top-right"><WorkflowToolbar /></Panel>
    </ReactFlow>
  );
}

function App() {
  return (
    <ReactFlowProvider>
      <DnDProvider>
        <Sidebar />
        <WorkflowCanvas />
        <ConfigPanel />
      </DnDProvider>
    </ReactFlowProvider>
  );
}
```

---

## Sources

- [Custom Nodes](https://reactflow.dev/learn/customization/custom-nodes) · [Handles](https://reactflow.dev/api-reference/components/handle)
- [Theming & CSS Vars](https://reactflow.dev/learn/customization/theming) · [Dark Mode](https://reactflow.dev/examples/styling/dark-mode)
- [Edge Animations](https://reactflow.dev/examples/edges/animating-edges) · [Animated SVG Edge](https://reactflow.dev/ui/components/animated-svg-edge)
- [Drag and Drop](https://reactflow.dev/examples/interaction/drag-and-drop)
- [Dagre Layout](https://reactflow.dev/examples/layout/dagre) · [ELK Layout](https://reactflow.dev/examples/layout/elkjs)
- [Performance](https://reactflow.dev/learn/advanced-use/performance) · [Synergy Codes Guide](https://www.synergycodes.com/webbook/guide-to-optimize-react-flow-project-performance)
- [Prevent Cycles](https://reactflow.dev/examples/interaction/prevent-cycles) · [Validation](https://reactflow.dev/examples/interaction/validation)
- [Context Menu](https://reactflow.dev/examples/interaction/context-menu) · [Sub Flows](https://reactflow.dev/examples/grouping/sub-flows)
- [MiniMap](https://reactflow.dev/api-reference/components/minimap) · [Controls](https://reactflow.dev/api-reference/components/controls)
- [ReactFlow API](https://reactflow.dev/api-reference/react-flow) · [React Flow UI](https://reactflow.dev/ui)
- [Edge Animation Perf](https://liambx.com/blog/tuning-edge-animations-reactflow-optimal-performance)
