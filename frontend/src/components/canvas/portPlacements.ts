import { CanvasNodeKind } from './canvasKinds'
import { Geometry } from '@/utils/geometry'
import type { Point, Rect, Side } from '@/utils/geometry'

// ============================================================================
// Port Placement Registry — Centralized Handle Configuration
// ============================================================================

type PortRole =
  | 'control-in'
  | 'control-out'
  | 'agents'
  | 'documents'
  | 'agent-input'
  | 'agent-output'
  | 'agent-documents'
  | 'document-input'

type PortPlacement = {
  readonly side: Side
  readonly role: PortRole
  readonly handleType: 'source' | 'target'
  readonly handleId: string | null
}

type NodePortConfig = {
  readonly kind: CanvasNodeKind
  readonly ports: readonly PortPlacement[]
}

// ── Registry Data ────────────────────────────────────────────────────

const PORT_CONFIGS: Readonly<Record<CanvasNodeKind, NodePortConfig>> = {
  [CanvasNodeKind.STEP]: {
    kind: CanvasNodeKind.STEP,
    ports: [
      { side: 'left', role: 'control-in', handleType: 'target', handleId: null },
      { side: 'right', role: 'control-out', handleType: 'source', handleId: null },
    ],
  },
  [CanvasNodeKind.PROTOCOL]: {
    kind: CanvasNodeKind.PROTOCOL,
    ports: [
      { side: 'left', role: 'control-in', handleType: 'target', handleId: null },
      { side: 'right', role: 'control-out', handleType: 'source', handleId: null },
      { side: 'top', role: 'agents', handleType: 'source', handleId: 'agents' },
      { side: 'top', role: 'documents', handleType: 'source', handleId: 'documents' },
    ],
  },
  [CanvasNodeKind.AGENT]: {
    kind: CanvasNodeKind.AGENT,
    ports: [
      { side: 'bottom', role: 'agent-input', handleType: 'target', handleId: 'agent-input' },
      { side: 'top', role: 'agent-output', handleType: 'source', handleId: 'agent-output' },
      { side: 'right', role: 'agent-documents', handleType: 'source', handleId: 'agent-documents' },
    ],
  },
  [CanvasNodeKind.CONTEXT]: {
    kind: CanvasNodeKind.CONTEXT,
    ports: [
      { side: 'bottom', role: 'control-out', handleType: 'source', handleId: null },
    ],
  },
  [CanvasNodeKind.INPUT]: {
    kind: CanvasNodeKind.INPUT,
    ports: [
      { side: 'bottom', role: 'control-out', handleType: 'source', handleId: null },
    ],
  },
  [CanvasNodeKind.DOCUMENT]: {
    kind: CanvasNodeKind.DOCUMENT,
    ports: [
      { side: 'bottom', role: 'document-input', handleType: 'target', handleId: 'document-input' },
    ],
  },
  [CanvasNodeKind.SUB_WORKFLOW]: {
    kind: CanvasNodeKind.SUB_WORKFLOW,
    ports: [
      { side: 'left', role: 'control-in', handleType: 'target', handleId: null },
      { side: 'right', role: 'control-out', handleType: 'source', handleId: null },
    ],
  },
}

// ── Query Functions ──────────────────────────────────────────────────

/** Get all port placements for a canvas node kind. */
const getPortConfig = (kind: CanvasNodeKind): NodePortConfig =>
  PORT_CONFIGS[kind]

/** Get ports on a specific side of a node kind. */
const getPortsOnSide = (kind: CanvasNodeKind, side: Side): readonly PortPlacement[] => {
  const ports: PortPlacement[] = []
  const config = PORT_CONFIGS[kind]
  const n = config.ports.length
  for (let i = 0; i < n; i++) {
    const port = config.ports[i]!
    if (port.side === side) ports.push(port)
  }
  return ports
}

/**
 * Compute the absolute pixel position of a port by its role.
 * When multiple ports share a side, they are evenly spaced along that side.
 * Returns `null` if the role is not found on this node kind.
 */
const getPortPosition = (kind: CanvasNodeKind, rect: Rect, role: PortRole): Point | null => {
  const config = PORT_CONFIGS[kind]

  // Find the target port and collect all ports on its side
  let targetPort: PortPlacement | null = null
  for (let i = 0; i < config.ports.length; i++) {
    if (config.ports[i]!.role === role) {
      targetPort = config.ports[i]!
      break
    }
  }
  if (!targetPort) return null

  const sameSide: PortPlacement[] = []
  let targetIndex = 0
  for (let i = 0; i < config.ports.length; i++) {
    const port = config.ports[i]!
    if (port.side === targetPort.side) {
      if (port.role === role) targetIndex = sameSide.length
      sameSide.push(port)
    }
  }

  // Single port → centered (fraction = 0.5). Multiple → evenly spaced.
  const fraction = sameSide.length === 1
    ? 0.5
    : (targetIndex + 1) / (sameSide.length + 1)

  return Geometry.pointAlongSide(rect, targetPort.side, fraction)
}

export { getPortConfig, getPortsOnSide, getPortPosition, PORT_CONFIGS }
export type { PortRole, PortPlacement, NodePortConfig }
