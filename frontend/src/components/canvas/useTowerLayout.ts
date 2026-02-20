import { useCallback, useEffect, useRef } from 'react'
import type { Node } from '@xyflow/react'
import { computeAllTowerPositions } from './layout'
import type { ProtocolDimensions, NodeDimensions } from './layout'
import { isVirtualNode, setStoredPosition } from './nodeResizeStorage'
import { isWorkforceStep } from './mappers/protocolGroups'
import { NODE_DIMENSIONS } from './nodeDimensions'
import { CanvasNodeKind } from './canvasKinds'
import { AGENT_DEFAULTS } from './CanvasNode/registry'
import type { StepNodeLookups } from './mappers/types'
import type { WorkflowStep } from '@/types/workflow'

type UseTowerLayoutResult = {
  restackTowers: () => void
}

/**
 * Identify protocol steps that have agent towers.
 */
const findProtocolStepIds = (
  steps: readonly WorkflowStep[],
  lookups: StepNodeLookups,
): Set<string> => {
  const ids = new Set<string>()
  for (const step of steps) {
    if (isWorkforceStep(step, lookups.protocolsByStep)) {
      ids.add(step.id)
    }
  }
  return ids
}

/**
 * Build protocol dimensions from current React Flow node state.
 */
const buildProtocolDimensions = (
  protocolStepIds: ReadonlySet<string>,
  nodes: readonly Node[],
): Map<string, ProtocolDimensions> => {
  const dims = new Map<string, ProtocolDimensions>()
  const defaultWidth = NODE_DIMENSIONS[CanvasNodeKind.PROTOCOL].defaultWidth

  for (const node of nodes) {
    if (!protocolStepIds.has(node.id)) continue
    dims.set(node.id, {
      x: node.position.x,
      y: node.position.y,
      width: node.measured?.width ?? (node.width ?? defaultWidth),
    })
  }

  return dims
}

/**
 * Build measured dimensions for all agent artifact nodes from React Flow state.
 */
const buildAgentDimensions = (
  nodes: readonly Node[],
): Map<string, NodeDimensions> => {
  const dims = new Map<string, NodeDimensions>()
  const defaultWidth = AGENT_DEFAULTS.DEFAULT_WIDTH
  const defaultHeight = AGENT_DEFAULTS.DEFAULT_HEIGHT

  for (const node of nodes) {
    if (!node.id.startsWith('agent-artifact-')) continue
    dims.set(node.id, {
      width: node.measured?.width ?? (node.width ?? defaultWidth),
      height: node.measured?.height ?? (node.height ?? defaultHeight),
    })
  }

  return dims
}

/**
 * Reactive tower layout hook.
 *
 * Watches for dimension/position changes on protocol and agent nodes.
 * When a change is detected, recomputes tower positions for all affected
 * protocols and updates agent node positions in React Flow.
 *
 * Uses actual measured node dimensions — the tallest agent in a tier
 * determines vertical spacing, and each agent's measured width is used
 * for horizontal centering.
 *
 * Also exposes `restackTowers()` for the manual "Auto Layout" button,
 * which re-stacks all towers from their protocols' current positions.
 */
const useTowerLayout = (
  steps: readonly WorkflowStep[],
  lookups: StepNodeLookups,
  getNodes: () => Node[],
  setNodes: (updater: (nodes: Node[]) => Node[]) => void,
): UseTowerLayoutResult => {
  const rafRef = useRef<number | null>(null)
  const prevSnapshotRef = useRef<string>('')

  const applyTowerPositions = useCallback((positions: ReadonlyMap<string, { x: number; y: number }>) => {
    if (positions.size === 0) return

    setNodes((current) =>
      current.map((node) => {
        const pos = positions.get(node.id)
        if (!pos) return node
        // Only update if position actually changed
        if (node.position.x === pos.x && node.position.y === pos.y) return node
        return { ...node, position: { x: pos.x, y: pos.y } }
      }),
    )

    // Persist virtual node positions
    for (const [nodeId, pos] of positions) {
      if (isVirtualNode(nodeId)) {
        setStoredPosition(nodeId, { x: Math.round(pos.x), y: Math.round(pos.y) })
      }
    }
  }, [setNodes])

  /**
   * Restack all towers from current protocol positions.
   * Called by the "Auto Layout" button and reactively on dimension changes.
   */
  const restackTowers = useCallback(() => {
    const protocolStepIds = findProtocolStepIds(steps, lookups)
    if (protocolStepIds.size === 0) return

    const nodes = getNodes()
    const protocolDims = buildProtocolDimensions(protocolStepIds, nodes)
    const agentDims = buildAgentDimensions(nodes)
    const positions = computeAllTowerPositions(protocolDims, lookups, agentDims)
    applyTowerPositions(positions)
  }, [steps, lookups, getNodes, applyTowerPositions])

  // Reactive: watch for protocol/agent dimension changes and restack
  useEffect(() => {
    const checkAndRestack = () => {
      const protocolStepIds = findProtocolStepIds(steps, lookups)
      if (protocolStepIds.size === 0) {
        prevSnapshotRef.current = ''
        return
      }

      const nodes = getNodes()
      const protocolDims = buildProtocolDimensions(protocolStepIds, nodes)
      const agentDims = buildAgentDimensions(nodes)

      // Serialize current state for comparison (protocol pos/size + agent sizes)
      const serialized = JSON.stringify([
        [...protocolDims.entries()].map(([id, d]) => [id, d.x, d.y, d.width]),
        [...agentDims.entries()].map(([id, d]) => [id, d.width, d.height]),
      ])

      if (serialized === prevSnapshotRef.current) return
      prevSnapshotRef.current = serialized

      const positions = computeAllTowerPositions(protocolDims, lookups, agentDims)
      applyTowerPositions(positions)
    }

    // Poll at animation frame rate during active interactions.
    // This catches resize and drag events without needing explicit callbacks.
    let active = true
    const tick = () => {
      if (!active) return
      checkAndRestack()
      rafRef.current = requestAnimationFrame(tick)
    }
    rafRef.current = requestAnimationFrame(tick)

    return () => {
      active = false
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current)
      }
    }
  }, [steps, lookups, getNodes, applyTowerPositions])

  return { restackTowers }
}

export { useTowerLayout }
