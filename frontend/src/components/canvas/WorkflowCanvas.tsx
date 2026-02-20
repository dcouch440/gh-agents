import { useCallback, useEffect, useMemo, useRef } from 'react'
import { useAutoSave } from '@/hooks'
import { ReactFlow, Background, MiniMap, useReactFlow, ReactFlowProvider, BackgroundVariant } from '@xyflow/react'
import type { OnSelectionChangeParams, Connection, OnNodesDelete, OnEdgesDelete, Edge } from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, batch, workflowStore, canvasStore, agentStore, outputSchemaStore, shareStore } from '@/stores'
import { toRFNodes, toRFEdges, toAgentEdges } from './mappers'
import { Collections } from '@/utils/collections'
import { nodeTypes } from './nodeTypes'
import { edgeTypes } from './edgeTypes'
import { usePackDrag } from './usePackDrag'
import { OptionTray } from './OptionTray'
import { CanvasContextMenu } from './CanvasContextMenu'
import { CANVAS } from './constants'
import { computeHighlightedProtocolIds } from './computeHighlightedProtocolIds'
import { useGroupHoverDelay } from './useGroupHoverDelay'
import { useCanvasSync } from './useCanvasSync'
import { useCanvasLookups } from './useCanvasLookups'
import { useCanvasFetch } from './useCanvasFetch'
import { ShareModeBanner } from './ShareModeBanner'
import { FocusModeOverlay } from '@/components/focus-mode'
import { useEnterFocusMode } from './useEnterFocusMode'
import { useCanvasKeyboard } from './useCanvasKeyboard'
import { useContextMenuState } from './useContextMenuState'
import { useTowerLayout } from './useTowerLayout'

function WorkflowCanvasInner() {
  const theme = useTheme()
  const { getNodes, setNodes, setEdges, fitView, screenToFlowPosition } = useReactFlow()
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)
  const agents = useStore(agentStore.store, agentStore.selectAll)
  const schemas = useStore(outputSchemaStore.store, outputSchemaStore.selectAll)
  const minimapVisible = useStore(canvasStore.store, canvasStore.selectMinimapVisible)
  const toolsByAgent = useStore(agentStore.store, agentStore.selectToolsByAgent)
  const stepProtocols = useStore(canvasStore.store, canvasStore.selectStepProtocols)
  const rosterByStep = useStore(workflowStore.store, workflowStore.selectRosterByStep)
  const { onNodeDragStart, onNodeDrag, onNodeDragStop } = usePackDrag(getNodes, setNodes)
  const stepsById = useMemo(() => Collections.keyBy(steps, (s) => s.id), [steps])
  const shareActive = useStore(shareStore.store, shareStore.selectActive)
  const { contextMenu, closeMenu, onPaneContextMenu, onNodeContextMenu, onCanvasMouseDown } = useContextMenuState(screenToFlowPosition)
  const initialFitDone = useRef(false)

  const autoSave = useAutoSave(true)

  useCanvasFetch(agents, steps)

  // Build lookup maps for node data enrichment
  const { lookups, protocolGroups, protocolsByStepLookup } = useCanvasLookups(
    steps,
    edges,
    agents,
    schemas,
    toolsByAgent,
    stepProtocols,
    rosterByStep,
  )

  // Map store data to RF format
  const rfNodes = useMemo(() => toRFNodes(steps, lookups), [steps, lookups])
  const nodePalette = theme.palette.nodePalette
  const rfEdges = useMemo(
    () => [...toRFEdges(edges, protocolGroups, protocolsByStepLookup, steps, nodePalette), ...toAgentEdges(steps, lookups, nodePalette)],
    [edges, protocolGroups, protocolsByStepLookup, steps, lookups, nodePalette],
  )

  // Push store updates into RF — only touch data + position, never clobber selection
  useCanvasSync(rfNodes, rfEdges, setNodes, setEdges)

  // Tower layout engine — reactively keeps agent towers stacked above their protocols
  const { restackTowers } = useTowerLayout(steps, lookups, getNodes, setNodes)

  // Fit to view on initial load
  useEffect(() => {
    if (steps.length > 0 && !initialFitDone.current) {
      initialFitDone.current = true
      setTimeout(() => {
        void fitView({ padding: CANVAS.FIT_VIEW_PADDING })
      }, 50)
    }
  }, [steps, fitView])

  // Selection sync: RF → canvasStore (read-only mirror for sidebar panels)
  // Read protocolStepId / isProtocol directly from node data so there are no stale closures
  const onSelectionChange = useCallback((params: OnSelectionChangeParams) => {
    if (shareStore.store.getState().active) return
    const nodeIds = Collections.mapBy(params.nodes, (n: { id: string }) => n.id)
    const protocolIds = computeHighlightedProtocolIds(params.nodes)
    batch(() => {
      canvasStore.selectSteps(nodeIds)
      canvasStore.selectEdges(Collections.mapBy(params.edges, (e: { id: string }) => e.id))
      canvasStore.setHighlightedProtocols(protocolIds)
    })
  }, [])

  // Edge validation — context nodes are source-only, no self-loops
  const isValidConnection = useCallback(
    (connection: Connection) => {
      if (connection.sourceHandle === 'agents') return false
      const targetStep = connection.target ? stepsById.get(connection.target) : undefined
      if (!targetStep) return false
      if (targetStep.execution_mode === 'context' || targetStep.execution_mode === 'input') return false
      if (connection.source === connection.target) return false
      return true
    },
    [stepsById],
  )

  // Edge creation
  const onConnect = useCallback((connection: Connection) => {
    if (!connection.source || !connection.target) return
    void workflowStore.addEdge({
      from_step_id: connection.source,
      to_step_id: connection.target,
    })
  }, [])

  // Edge reconnection (drag from handle to detach/reconnect)
  const onReconnect = useCallback((oldEdge: Edge, newConnection: Connection) => {
    if (!newConnection.source || !newConnection.target) return

    // Delete old edge
    void workflowStore.removeEdge(oldEdge.id)

    // Create new edge with updated source/target
    void workflowStore.addEdge({
      from_step_id: newConnection.source,
      to_step_id: newConnection.target,
    })
  }, [])

  // Node deletion
  const onNodesDelete: OnNodesDelete = useCallback((deleted) => {
    for (const node of deleted) {
      if (node.id.startsWith('agent-artifact-')) continue
      void workflowStore.deleteStep(node.id)
    }
  }, [])

  // Edge deletion
  const onEdgesDelete: OnEdgesDelete = useCallback((deleted) => {
    for (const edge of deleted) {
      if (edge.id.startsWith('agent-edge-') || edge.id.startsWith('agent-dep-')) continue
      void workflowStore.removeEdge(edge.id)
    }
  }, [])

  // Close context menu on pane or node click — share mode intercepts
  const onPaneClick = useCallback(() => {
    if (shareStore.store.getState().active) {
      shareStore.cancelShare()
      return
    }
    closeMenu()
  }, [closeMenu])

  const onNodeClick = useCallback(
    (_event: React.MouseEvent, node: { id: string }) => {
      if (shareStore.store.getState().active) {
        if (node.id.startsWith('agent-artifact-')) return
        shareStore.commitShare(node.id)
        return
      }
      closeMenu()
    },
    [closeMenu],
  )

  // Protocol hover tracking for group highlighting.
  // Self-hover is instant; group hover triggers after a 300ms delay.
  const { onNodeMouseEnter, onNodeMouseLeave } = useGroupHoverDelay()

  // Global keyboard shortcuts (ESC → cancel share, Alt+F → focus mode)
  const enterFocusMode = useEnterFocusMode()
  useCanvasKeyboard(shareActive, enterFocusMode)

  return (
    <Box
      data-testid="workflow-canvas"
      onMouseDown={onCanvasMouseDown}
      sx={{
        width: '100%',
        height: '100%',
        position: 'relative',
        outline: 'none',
        '&::after': {
          content: '""',
          position: 'absolute',
          inset: 0,
          pointerEvents: 'none',
          zIndex: 0,
        },
        '& .react-flow': {
          '--xy-background-color': theme.palette.custom.canvasBg,
          '--xy-node-background-color': 'transparent',
          '--xy-node-border': 'none',
          '--xy-node-border-radius': '8px',
          '--xy-minimap-background-color': theme.palette.custom.minimapBg,
          '--xy-minimap-mask-background-color': theme.palette.custom.minimapMask,
        },
        '& .react-flow__node.dragging': {
          willChange: 'transform',
        },
      }}
    >
      <ReactFlow
        defaultNodes={rfNodes}
        defaultEdges={rfEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onSelectionChange={onSelectionChange}
        onNodeDragStart={onNodeDragStart}
        onNodeDrag={onNodeDrag}
        onNodeDragStop={onNodeDragStop}
        isValidConnection={isValidConnection}
        onConnect={onConnect}
        onReconnect={onReconnect}
        onNodesDelete={onNodesDelete}
        onEdgesDelete={onEdgesDelete}
        onPaneContextMenu={onPaneContextMenu}
        onNodeContextMenu={onNodeContextMenu}
        onPaneClick={onPaneClick}
        onNodeClick={onNodeClick}
        onNodeMouseEnter={onNodeMouseEnter}
        onNodeMouseLeave={onNodeMouseLeave}
        deleteKeyCode={['Backspace', 'Delete']}
        multiSelectionKeyCode="Shift"
        reconnectRadius={20}
        minZoom={0.25}
        maxZoom={4}
        snapToGrid
        snapGrid={[CANVAS.GRID_SIZE, CANVAS.GRID_SIZE]}
        fitView={false}
        proOptions={{ hideAttribution: true }}
      >
        <Background
          id="stitch-lines"
          variant={BackgroundVariant.Lines}
          gap={CANVAS.GRID_SIZE}
          lineWidth={1}
          color={theme.palette.custom.gridLineColor}
        />
        <Background
          id="stitch-dots"
          variant={BackgroundVariant.Dots}
          gap={CANVAS.GRID_SIZE}
          size={1.5}
          color={theme.palette.custom.gridDotColor}
        />
        {minimapVisible && (
          <MiniMap
            nodeStrokeColor={theme.palette.primary.main}
            nodeColor={theme.palette.custom.surfaceBg}
            nodeBorderRadius={8}
            maskColor={theme.palette.custom.minimapMask}
          />
        )}
      </ReactFlow>
      <OptionTray autoSaveFlush={autoSave.flush} autoSaveSaving={autoSave.saving} onAutoLayout={restackTowers} />
      {shareActive && <ShareModeBanner />}
      <CanvasContextMenu position={contextMenu} onClose={closeMenu} />
      <FocusModeOverlay />
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
