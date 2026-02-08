import { useCallback, useMemo, useState, useEffect, useRef } from 'react'
import {
  ReactFlow,
  Background,
  MiniMap,
  useReactFlow,
  ReactFlowProvider,
  BackgroundVariant,
  applyNodeChanges,
  applyEdgeChanges,
} from '@xyflow/react'
import type {
  OnSelectionChangeParams,
  Connection,
  OnNodesDelete,
  OnEdgesDelete,
  NodeChange,
  EdgeChange,
  Node,
  Edge,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import Box from '@mui/material/Box'
import { useStore, workflowStore, canvasStore } from '@/stores'
import { toRFNodes, toRFEdges } from './mappers'
import { nodeTypes } from './nodeTypes'
import { edgeTypes } from './edgeTypes'
import { usePositionPersist } from './usePositionPersist'
import { CanvasContextMenu } from './CanvasContextMenu'
import type { MenuPosition } from './CanvasContextMenu'
import { CANVAS } from './constants'

function WorkflowCanvasInner() {
  const reactFlowInstance = useReactFlow()
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)
  const selectedEdgeIds = useStore(canvasStore.store, canvasStore.selectSelectedEdgeIds)
  const minimapVisible = useStore(canvasStore.store, canvasStore.selectMinimapVisible)
  const { onNodeDragStop } = usePositionPersist()
  const [contextMenu, setContextMenu] = useState<MenuPosition>(null)
  const initialFitDone = useRef(false)

  // Store-derived RF data
  const rfNodesFromStore = useMemo(
    () => toRFNodes(steps, selectedStepIds),
    [steps, selectedStepIds],
  )
  const rfEdgesFromStore = useMemo(
    () => toRFEdges(edges, selectedEdgeIds),
    [edges, selectedEdgeIds],
  )

  // Local state for smooth dragging — synced from store
  const [localNodes, setLocalNodes] = useState<Node[]>([])
  const [localEdges, setLocalEdges] = useState<Edge[]>([])

  useEffect(() => {
    setLocalNodes(rfNodesFromStore)
  }, [rfNodesFromStore])

  useEffect(() => {
    setLocalEdges(rfEdgesFromStore)
  }, [rfEdgesFromStore])

  // Fit to view on initial load
  useEffect(() => {
    if (steps.length > 0 && !initialFitDone.current) {
      initialFitDone.current = true
      setTimeout(() => {
        void reactFlowInstance.fitView({ padding: CANVAS.FIT_VIEW_PADDING })
      }, 50)
    }
  }, [steps, reactFlowInstance])

  // Handle node position changes during drag (local only)
  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setLocalNodes((nds) => applyNodeChanges(changes, nds))
  }, [])

  // Handle edge changes (local only)
  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    setLocalEdges((eds) => applyEdgeChanges(changes, eds))
  }, [])

  // Selection sync: RF → canvasStore
  const onSelectionChange = useCallback((params: OnSelectionChangeParams) => {
    const nodeIds = params.nodes.map((n) => n.id)
    const edgeIds = params.edges.map((e) => e.id)
    canvasStore.selectSteps(nodeIds)
    canvasStore.selectEdges(edgeIds)
  }, [])

  // Edge creation
  const onConnect = useCallback((connection: Connection) => {
    if (!connection.source || !connection.target) return
    void workflowStore.addEdge({
      from_step_id: connection.source,
      to_step_id: connection.target,
    })
  }, [])

  // Node deletion
  const onNodesDelete: OnNodesDelete = useCallback((deleted) => {
    for (const node of deleted) {
      void workflowStore.deleteStep(node.id)
    }
  }, [])

  // Edge deletion
  const onEdgesDelete: OnEdgesDelete = useCallback((deleted) => {
    for (const edge of deleted) {
      void workflowStore.removeEdge(edge.id)
    }
  }, [])

  // Context menu (right-click on pane)
  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault()
      const flowPosition = reactFlowInstance.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      })
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        flowX: flowPosition.x,
        flowY: flowPosition.y,
      })
    },
    [reactFlowInstance],
  )

  // Close context menu on pane click
  const onPaneClick = useCallback(() => {
    setContextMenu(null)
  }, [])

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        position: 'relative',
        '& .react-flow': {
          '--xy-background-color': 'transparent',
          '--xy-node-background-color': 'transparent',
          '--xy-node-border': 'none',
          '--xy-node-border-radius': '12px',
          '--xy-minimap-background-color': 'rgba(6, 10, 16, 0.9)',
          '--xy-minimap-mask-background-color': 'rgba(0, 0, 0, 0.7)',
        },
      }}
    >
      <ReactFlow
        nodes={localNodes}
        edges={localEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onSelectionChange={onSelectionChange}
        onNodeDragStop={onNodeDragStop}
        onConnect={onConnect}
        onNodesDelete={onNodesDelete}
        onEdgesDelete={onEdgesDelete}
        onPaneContextMenu={onPaneContextMenu}
        onPaneClick={onPaneClick}
        deleteKeyCode={['Backspace', 'Delete']}
        multiSelectionKeyCode="Shift"
        snapToGrid
        snapGrid={[CANVAS.GRID_SIZE, CANVAS.GRID_SIZE]}
        fitView={false}
        proOptions={{ hideAttribution: true }}
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
      <CanvasContextMenu
        position={contextMenu}
        onClose={() => {
          setContextMenu(null)
        }}
      />
    </Box>
  )
}

function WorkflowCanvas() {
  return (
    <ReactFlowProvider>
      <WorkflowCanvasInner />
    </ReactFlowProvider>
  )
}

export { WorkflowCanvas }
