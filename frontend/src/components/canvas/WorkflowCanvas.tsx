import {useCallback, useEffect, useMemo, useRef, useState} from "react";
import {
  ReactFlow,
  Background,
  MiniMap,
  useReactFlow,
  ReactFlowProvider,
  BackgroundVariant,
} from "@xyflow/react";
import type {
  OnSelectionChangeParams,
  Connection,
  OnNodesDelete,
  OnEdgesDelete,
  Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import Box from "@mui/material/Box";
import {
  useStore,
  workflowStore,
  canvasStore,
  layoutStore,
  agentStore,
  outputSchemaStore,
} from "@/stores";
import {toRFNodes, toRFEdges} from "./mappers";
import type {StepNodeLookups} from "./mappers";
import {nodeTypes} from "./nodeTypes";
import {edgeTypes} from "./edgeTypes";
import {usePositionPersist} from "./usePositionPersist";
import {CanvasToolbar} from "./CanvasToolbar";
import {CanvasContextMenu} from "./CanvasContextMenu";
import type {MenuPosition} from "./CanvasContextMenu";
import {CANVAS} from "./constants";

function WorkflowCanvasInner() {
  const {setNodes, setEdges, fitView, screenToFlowPosition} = useReactFlow();
  const steps = useStore(workflowStore.store, workflowStore.selectSteps);
  const edges = useStore(workflowStore.store, workflowStore.selectEdges);
  const agents = useStore(agentStore.store, agentStore.selectAll);
  const schemas = useStore(
    outputSchemaStore.store,
    outputSchemaStore.selectAll,
  );
  const minimapVisible = useStore(
    canvasStore.store,
    canvasStore.selectMinimapVisible,
  );
  const {onNodeDragStop} = usePositionPersist();
  const [contextMenu, setContextMenu] = useState<MenuPosition>(null);
  const initialFitDone = useRef(false);

  // Build lookup maps for node data enrichment (split to avoid rebuilding stable maps on step changes)
  const agentLookup = useMemo(
    () =>
      new Map(agents.map((a) => [a.id, {name: a.name, model_id: a.model_id}])),
    [agents],
  );
  const schemaLookup = useMemo(
    () => new Map(schemas.map((s) => [s.id, {name: s.name}])),
    [schemas],
  );
  const stepNameLookup = useMemo(
    () => new Map(steps.map((s) => [s.id, s.name ?? s.execution_mode])),
    [steps],
  );
  const lookups = useMemo(
    (): StepNodeLookups => ({
      agents: agentLookup,
      outputSchemas: schemaLookup,
      stepNames: stepNameLookup,
      edges,
    }),
    [agentLookup, schemaLookup, stepNameLookup, edges],
  );

  // Map store data to RF format
  const rfNodes = useMemo(() => toRFNodes(steps, lookups), [steps, lookups]);
  const rfEdges = useMemo(() => toRFEdges(edges), [edges]);

  // Push store updates into RF — only touch data + position, never clobber selection
  useEffect(() => {
    setNodes((current) => {
      const currentIds = new Set(current.map((n) => n.id));
      const newIds = new Set(rfNodes.map((n) => n.id));

      const hasStructuralChange =
        rfNodes.some((n) => !currentIds.has(n.id)) ||
        current.some((n) => !newIds.has(n.id));

      if (hasStructuralChange) {
        // Nodes added/removed — full replacement, preserve selection
        const selMap = new Map(current.map((n) => [n.id, n.selected ?? false]));
        return rfNodes.map((n) => ({
          ...n,
          selected: selMap.get(n.id) ?? false,
        }));
      }

      // Data-only change — update data + position, never touch selection
      const newDataMap = new Map(rfNodes.map((n) => [n.id, n]));
      return current.map((n) => {
        const updated = newDataMap.get(n.id);
        if (!updated) return n;
        if (n.data === updated.data && n.position === updated.position)
          return n;
        return {...n, data: updated.data, position: updated.position};
      });
    });
  }, [rfNodes, setNodes]);

  useEffect(() => {
    setEdges((current) => {
      const selMap = new Map(current.map((e) => [e.id, e.selected ?? false]));
      return rfEdges.map((e) => ({
        ...e,
        selected: selMap.get(e.id) ?? false,
      }));
    });
  }, [rfEdges, setEdges]);

  // Fit to view on initial load
  useEffect(() => {
    if (steps.length > 0 && !initialFitDone.current) {
      initialFitDone.current = true;
      setTimeout(() => {
        void fitView({padding: CANVAS.FIT_VIEW_PADDING});
      }, 50);
    }
  }, [steps, fitView]);

  // Selection sync: RF → canvasStore (read-only mirror for sidebar panels)
  const onSelectionChange = useCallback((params: OnSelectionChangeParams) => {
    canvasStore.selectSteps(params.nodes.map((n) => n.id));
    canvasStore.selectEdges(params.edges.map((e) => e.id));
    if (params.nodes.length > 0 || params.edges.length > 0) {
      layoutStore.openRightPanelIfClosed("properties");
    }
  }, []);

  // Edge creation
  const onConnect = useCallback((connection: Connection) => {
    if (!connection.source || !connection.target) return;
    void workflowStore.addEdge({
      from_step_id: connection.source,
      to_step_id: connection.target,
    });
  }, []);

  // Edge reconnection (drag from handle to detach/reconnect)
  const onReconnect = useCallback(
    (oldEdge: Edge, newConnection: Connection) => {
      if (!newConnection.source || !newConnection.target) return;

      // Delete old edge
      void workflowStore.removeEdge(oldEdge.id);

      // Create new edge with updated source/target
      void workflowStore.addEdge({
        from_step_id: newConnection.source,
        to_step_id: newConnection.target,
      });
    },
    [],
  );

  // Node deletion
  const onNodesDelete: OnNodesDelete = useCallback((deleted) => {
    for (const node of deleted) {
      void workflowStore.deleteStep(node.id);
    }
  }, []);

  // Edge deletion
  const onEdgesDelete: OnEdgesDelete = useCallback((deleted) => {
    for (const edge of deleted) {
      void workflowStore.removeEdge(edge.id);
    }
  }, []);

  // Context menu (right-click on pane)
  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      const flowPosition = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        flowX: flowPosition.x,
        flowY: flowPosition.y,
      });
    },
    [screenToFlowPosition],
  );

  // Context menu (right-click on node)
  const onNodeContextMenu = useCallback(
    (
      event: React.MouseEvent,
      node: {id: string; position: {x: number; y: number}},
    ) => {
      event.preventDefault();
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        flowX: node.position.x,
        flowY: node.position.y,
        nodeId: node.id,
      });
    },
    [],
  );

  // Close context menu on pane or node click
  const onPaneClick = useCallback(() => {
    setContextMenu(null);
  }, []);

  const onNodeClick = useCallback(() => {
    setContextMenu(null);
  }, []);

  return (
    <Box
      sx={{
        width: "100%",
        height: "100%",
        position: "relative",
        outline: "none",
        "& .react-flow": {
          "--xy-background-color": "transparent",
          "--xy-node-background-color": "transparent",
          "--xy-node-border": "none",
          "--xy-node-border-radius": "12px",
          "--xy-minimap-background-color": "rgba(6, 10, 16, 0.9)",
          "--xy-minimap-mask-background-color": "rgba(0, 0, 0, 0.7)",
        },
      }}
    >
      <ReactFlow
        defaultNodes={rfNodes}
        defaultEdges={rfEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onSelectionChange={onSelectionChange}
        onNodeDragStop={onNodeDragStop}
        onConnect={onConnect}
        onReconnect={onReconnect}
        onNodesDelete={onNodesDelete}
        onEdgesDelete={onEdgesDelete}
        onPaneContextMenu={onPaneContextMenu}
        onNodeContextMenu={onNodeContextMenu}
        onPaneClick={onPaneClick}
        onNodeClick={onNodeClick}
        deleteKeyCode={["Backspace", "Delete"]}
        multiSelectionKeyCode="Shift"
        reconnectRadius={20}
        snapToGrid
        snapGrid={[CANVAS.GRID_SIZE, CANVAS.GRID_SIZE]}
        fitView={false}
        proOptions={{hideAttribution: true}}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={CANVAS.GRID_SIZE}
          size={1}
          color={CANVAS.GRID_DOT_COLOR}
        />
        {minimapVisible && (
          <MiniMap
            nodeStrokeColor="#3b82f6"
            nodeColor="#111318"
            nodeBorderRadius={8}
            maskColor="rgba(0, 0, 0, 0.7)"
          />
        )}
      </ReactFlow>
      <CanvasToolbar />
      <CanvasContextMenu
        position={contextMenu}
        onClose={() => {
          setContextMenu(null);
        }}
      />
    </Box>
  );
}

function WorkflowCanvas() {
  return (
    <ReactFlowProvider>
      <WorkflowCanvasInner />
    </ReactFlowProvider>
  );
}

export {WorkflowCanvas};
