import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ReactFlow, Background, MiniMap, useReactFlow, ReactFlowProvider, BackgroundVariant } from '@xyflow/react'
import type { OnSelectionChangeParams, Connection, OnNodesDelete, OnEdgesDelete, Edge } from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, batch, workflowStore, canvasStore, agentStore, outputSchemaStore, protocolStore } from '@/stores'
import { toRFNodes, toRFEdges, toDocumentEdges, computeProtocolGroups } from './mappers'
import type { StepNodeLookups } from './mappers'
import { Collections } from '@/utils/collections'
import { nodeTypes } from './nodeTypes'
import { edgeTypes } from './edgeTypes'
import { usePackDrag } from './usePackDrag'
import { OptionTray } from './OptionTray'
import { CanvasContextMenu } from './CanvasContextMenu'
import type { MenuPosition } from './CanvasContextMenu'
import { CANVAS } from './constants'
import { computeHighlightedProtocolIds } from './computeHighlightedProtocolIds'
import { useGroupHoverDelay } from './useGroupHoverDelay'
import { useCanvasSync } from './useCanvasSync'

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
  const documentDefsByStep = useStore(workflowStore.store, workflowStore.selectDocumentDefsByStep)
  const { onNodeDragStart, onNodeDrag, onNodeDragStop } = usePackDrag(getNodes, setNodes)
  const [contextMenu, setContextMenu] = useState<MenuPosition>(null)
  const initialFitDone = useRef(false)
  const fetchedToolAgentIds = useRef(new Set<string>())
  const fetchedDocDefStepIds = useRef(new Set<string>())

  // Fetch tools for agents not yet fetched
  useEffect(() => {
    agents.forEach((agent) => {
      if (!fetchedToolAgentIds.current.has(agent.id)) {
        fetchedToolAgentIds.current.add(agent.id)
        void agentStore.fetchTools(agent.id)
      }
    })
  }, [agents])

  // Fetch document defs for documenter steps not yet fetched
  useEffect(() => {
    steps.forEach((step) => {
      if (step.execution_mode === 'documenter' && !fetchedDocDefStepIds.current.has(step.id)) {
        fetchedDocDefStepIds.current.add(step.id)
        void workflowStore.fetchDocumentDefs(step.id)
      }
    })
  }, [steps])

  useEffect(() => {
    void protocolStore.fetchAll()
    void protocolStore.fetchTypes()
  }, [])

  // Build lookup maps for node data enrichment (split to avoid rebuilding stable maps on step changes)
  const agentLookup = useMemo(
    () =>
      Collections.toLookupMap(
        agents,
        (a) => a.id,
        (a) => ({ name: a.name, model_id: a.model_id }),
      ),
    [agents],
  )
  const schemaLookup = useMemo(
    () =>
      Collections.toLookupMap(
        schemas,
        (s) => s.id,
        (s) => ({ name: s.name }),
      ),
    [schemas],
  )
  const stepNameLookup = useMemo(
    () =>
      Collections.toLookupMap(
        steps,
        (s) => s.id,
        (s) => s.name ?? s.execution_mode,
      ),
    [steps],
  )
  // todo: why use map here?
  const toolsByAgentLookup = useMemo(
    () =>
      Collections.toLookupMap(
        agents,
        (a) => a.id,
        (a) => {
          const tools = toolsByAgent[a.id] ?? []
          return Collections.mapBy(tools, (t) => t.name)
        },
      ),
    [agents, toolsByAgent],
  )
  const protocolsByStepLookup = useMemo(
    () =>
      Collections.toLookupMap(
        Object.entries(stepProtocols),
        ([stepId]) => stepId,
        ([, link]) => ({
          protocol_type: link.protocolType,
          name: link.protocolName,
          portNames: link.portNames,
        }),
      ),
    [stepProtocols],
  )
  const protocolGroups = useMemo(
    () => computeProtocolGroups(steps, edges, protocolsByStepLookup),
    [steps, edges, protocolsByStepLookup],
  )
  const lookups = useMemo(
    (): StepNodeLookups => ({
      agents: agentLookup,
      outputSchemas: schemaLookup,
      stepNames: stepNameLookup,
      edges,
      toolsByAgent: toolsByAgentLookup,
      protocolsByStep: protocolsByStepLookup,
      documentDefsByStep,
      protocolGroups,
    }),
    [agentLookup, schemaLookup, stepNameLookup, edges, toolsByAgentLookup, protocolsByStepLookup, documentDefsByStep, protocolGroups],
  )

  // Map store data to RF format
  const rfNodes = useMemo(() => toRFNodes(steps, lookups), [steps, lookups])
  const rfEdges = useMemo(() => [...toRFEdges(edges, protocolGroups, protocolsByStepLookup, steps), ...toDocumentEdges(steps, lookups)], [edges, protocolGroups, protocolsByStepLookup, steps, lookups])

  // Push store updates into RF — only touch data + position, never clobber selection
  useCanvasSync(rfNodes, rfEdges, setNodes, setEdges)

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
      if (connection.sourceHandle === 'documents') return false
      const targetStep = steps.find((s) => s.id === connection.target)
      if (!targetStep) return false
      if (targetStep.execution_mode === 'context') return false
      if (connection.source === connection.target) return false
      return true
    },
    [steps],
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
      if (node.id.startsWith('doc-artifact-')) continue
      void workflowStore.deleteStep(node.id)
    }
  }, [])

  // Edge deletion
  const onEdgesDelete: OnEdgesDelete = useCallback((deleted) => {
    for (const edge of deleted) {
      if (edge.id.startsWith('doc-edge-')) continue
      void workflowStore.removeEdge(edge.id)
    }
  }, [])

  // Context menu (right-click on pane)
  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault()
      const flowPosition = screenToFlowPosition({
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
    [screenToFlowPosition],
  )

  // Context menu (right-click on node)
  const onNodeContextMenu = useCallback((event: React.MouseEvent, node: { id: string; position: { x: number; y: number } }) => {
    event.preventDefault()
    setContextMenu({
      x: event.clientX,
      y: event.clientY,
      flowX: node.position.x,
      flowY: node.position.y,
      nodeId: node.id,
    })
  }, [])

  // Close context menu on pane or node click
  const onPaneClick = useCallback(() => {
    setContextMenu(null)
  }, [])

  const onNodeClick = useCallback(() => {
    setContextMenu(null)
  }, [])

  // Protocol hover tracking for group highlighting.
  // Self-hover is instant; group hover triggers after a 300ms delay.
  const { onNodeMouseEnter, onNodeMouseLeave } = useGroupHoverDelay()

  const onCanvasMouseDown = useCallback(() => {
    setContextMenu(null)
  }, [])

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
          background: `radial-gradient(ellipse at 50% 50%, transparent 60%, ${theme.palette.custom.canvasVignette})`,
        },
        '& .react-flow': {
          '--xy-background-color': 'transparent',
          '--xy-node-background-color': 'transparent',
          '--xy-node-border': 'none',
          '--xy-node-border-radius': '12px',
          '--xy-minimap-background-color': theme.palette.custom.minimapBg,
          '--xy-minimap-mask-background-color': theme.palette.custom.minimapMask,
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
            nodeColor={theme.palette.background.paper}
            nodeBorderRadius={8}
            maskColor={theme.palette.custom.minimapMask}
          />
        )}
      </ReactFlow>
      <OptionTray />
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
