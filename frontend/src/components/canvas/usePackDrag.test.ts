import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import type { Node } from '@xyflow/react'
import { resolvePackMembers, usePackDrag } from './usePackDrag'
import type { PackNode } from './usePackDrag'
import { CanvasNodeKind } from './canvasKinds'

const { mockUpdateStep } = vi.hoisted(() => ({
  mockUpdateStep: vi.fn(),
}))

vi.mock('@/stores', () => ({
  workflowStore: {
    updateStep: mockUpdateStep,
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

const makePackNode = (id: string, kind: CanvasNodeKind, protocolStepId: string | null = null): PackNode => ({
  id,
  kind,
  protocolStepId,
})

const RF_TYPE_TO_KIND: Record<string, CanvasNodeKind> = {
  contextNode: CanvasNodeKind.CONTEXT,
  documentNode: CanvasNodeKind.DOCUMENT,
  documenterNode: CanvasNodeKind.PROTOCOL,
  stepNode: CanvasNodeKind.STEP,
}

const makeRFNode = (id: string, type: string, x: number, y: number, protocolStepId: string | null = null): Node => ({
  id,
  type,
  position: { x, y },
  data: { kind: RF_TYPE_TO_KIND[type] ?? CanvasNodeKind.STEP, protocolStepId },
})

describe('resolvePackMembers', () => {
  describe('protocol drag — pack resolution', () => {
    it('returns all hover-eligible members belonging to the dragged protocol', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-1', CanvasNodeKind.PROTOCOL, null),
        makePackNode('doc-1', CanvasNodeKind.DOCUMENT, 'proto-1'),
        makePackNode('doc-2', CanvasNodeKind.DOCUMENT, 'proto-1'),
        makePackNode('ctx-1', CanvasNodeKind.CONTEXT, 'proto-1'),
      ]

      const result = resolvePackMembers('proto-1', nodes)

      expect(result).toEqual(new Set(['doc-1', 'doc-2', 'ctx-1']))
    })

    it('ignores step nodes even if they reference the protocol', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-1', CanvasNodeKind.PROTOCOL, null),
        makePackNode('step-1', CanvasNodeKind.STEP, 'proto-1'),
        makePackNode('doc-1', CanvasNodeKind.DOCUMENT, 'proto-1'),
      ]

      const result = resolvePackMembers('proto-1', nodes)

      expect(result).toEqual(new Set(['doc-1']))
      expect(result.has('step-1')).toBe(false)
    })

    it('ignores nodes belonging to a different protocol', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-A', CanvasNodeKind.PROTOCOL, null),
        makePackNode('proto-B', CanvasNodeKind.PROTOCOL, null),
        makePackNode('doc-1', CanvasNodeKind.DOCUMENT, 'proto-A'),
        makePackNode('doc-2', CanvasNodeKind.DOCUMENT, 'proto-B'),
      ]

      const result = resolvePackMembers('proto-A', nodes)

      expect(result).toEqual(new Set(['doc-1']))
      expect(result.has('doc-2')).toBe(false)
    })

    it('returns empty set when protocol has no members', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-1', CanvasNodeKind.PROTOCOL, null),
        makePackNode('step-1', CanvasNodeKind.STEP, null),
      ]

      const result = resolvePackMembers('proto-1', nodes)

      expect(result.size).toBe(0)
    })

    it('excludes the dragged protocol node from the result', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-1', CanvasNodeKind.PROTOCOL, null),
        makePackNode('doc-1', CanvasNodeKind.DOCUMENT, 'proto-1'),
      ]

      const result = resolvePackMembers('proto-1', nodes)

      expect(result.has('proto-1')).toBe(false)
      expect(result).toEqual(new Set(['doc-1']))
    })
  })

  describe('non-protocol drag — solo movement', () => {
    it('context node returns empty set', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-1', CanvasNodeKind.PROTOCOL, null),
        makePackNode('ctx-1', CanvasNodeKind.CONTEXT, 'proto-1'),
        makePackNode('doc-1', CanvasNodeKind.DOCUMENT, 'proto-1'),
      ]

      const result = resolvePackMembers('ctx-1', nodes)

      expect(result.size).toBe(0)
    })

    it('document node returns empty set', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-1', CanvasNodeKind.PROTOCOL, null),
        makePackNode('doc-1', CanvasNodeKind.DOCUMENT, 'proto-1'),
        makePackNode('doc-2', CanvasNodeKind.DOCUMENT, 'proto-1'),
      ]

      const result = resolvePackMembers('doc-1', nodes)

      expect(result.size).toBe(0)
    })

    it('step node returns empty set', () => {
      const nodes: PackNode[] = [
        makePackNode('step-1', CanvasNodeKind.STEP, null),
        makePackNode('step-2', CanvasNodeKind.STEP, null),
      ]

      const result = resolvePackMembers('step-1', nodes)

      expect(result.size).toBe(0)
    })
  })

  describe('edge cases', () => {
    it('unknown node ID returns empty set', () => {
      const nodes: PackNode[] = [
        makePackNode('proto-1', CanvasNodeKind.PROTOCOL, null),
        makePackNode('doc-1', CanvasNodeKind.DOCUMENT, 'proto-1'),
      ]

      const result = resolvePackMembers('nonexistent', nodes)

      expect(result.size).toBe(0)
    })

    it('empty nodes array returns empty set', () => {
      const result = resolvePackMembers('any-id', [])

      expect(result.size).toBe(0)
    })
  })
})

describe('usePackDrag', () => {
  it('returns onNodeDragStart, onNodeDrag, and onNodeDragStop callbacks', () => {
    const getNodes = vi.fn<() => Node[]>(() => [])
    const setNodes = vi.fn()

    const { result } = renderHook(() => usePackDrag(getNodes, setNodes))

    expect(typeof result.current.onNodeDragStart).toBe('function')
    expect(typeof result.current.onNodeDrag).toBe('function')
    expect(typeof result.current.onNodeDragStop).toBe('function')
  })

  it('moves pack members when dragging a protocol node', () => {
    const rfNodes: Node[] = [
      makeRFNode('proto-1', 'documenterNode', 100, 100, null),
      makeRFNode('doc-1', 'documentNode', 200, 50, 'proto-1'),
      makeRFNode('ctx-1', 'contextNode', 300, 50, 'proto-1'),
    ]
    const getNodes = vi.fn<() => Node[]>(() => rfNodes)
    const setNodes = vi.fn()
    const mockEvent = {} as React.MouseEvent

    const { result } = renderHook(() => usePackDrag(getNodes, setNodes))

    // Start drag
    act(() => {
      result.current.onNodeDragStart(mockEvent, rfNodes[0]!)
    })

    // Drag protocol node by (50, 30)
    const draggedNode = { ...rfNodes[0]!, position: { x: 150, y: 130 } }
    act(() => {
      result.current.onNodeDrag(mockEvent, draggedNode)
    })

    expect(setNodes).toHaveBeenCalledTimes(1)
    const updater = setNodes.mock.calls[0]![0] as (nodes: Node[]) => Node[]
    const updated = updater(rfNodes)

    // doc-1: 200+50=250, 50+30=80
    const doc = updated.find((n) => n.id === 'doc-1')!
    expect(doc.position).toEqual({ x: 250, y: 80 })

    // ctx-1: 300+50=350, 50+30=80
    const ctx = updated.find((n) => n.id === 'ctx-1')!
    expect(ctx.position).toEqual({ x: 350, y: 80 })

    // proto-1 unchanged by setNodes (RF moves it)
    const proto = updated.find((n) => n.id === 'proto-1')!
    expect(proto.position).toEqual({ x: 100, y: 100 })
  })

  it('does not call setNodes when dragging a solo document node', () => {
    const rfNodes: Node[] = [
      makeRFNode('proto-1', 'documenterNode', 100, 100, null),
      makeRFNode('doc-1', 'documentNode', 200, 50, 'proto-1'),
    ]
    const getNodes = vi.fn<() => Node[]>(() => rfNodes)
    const setNodes = vi.fn()
    const mockEvent = {} as React.MouseEvent

    const { result } = renderHook(() => usePackDrag(getNodes, setNodes))

    // Start drag on document (not protocol)
    act(() => {
      result.current.onNodeDragStart(mockEvent, rfNodes[1]!)
    })

    // Drag
    const draggedNode = { ...rfNodes[1]!, position: { x: 250, y: 80 } }
    act(() => {
      result.current.onNodeDrag(mockEvent, draggedNode)
    })

    // setNodes should not be called — no pack members
    expect(setNodes).not.toHaveBeenCalled()
  })

  it('persists all pack member positions on drag stop', () => {
    const rfNodes: Node[] = [
      makeRFNode('proto-1', 'documenterNode', 100, 100, null),
      makeRFNode('doc-1', 'documentNode', 250, 80, 'proto-1'),
      makeRFNode('ctx-1', 'contextNode', 350, 80, 'proto-1'),
    ]
    const getNodes = vi.fn<() => Node[]>(() => rfNodes)
    const setNodes = vi.fn()
    const mockEvent = {} as React.MouseEvent

    const { result } = renderHook(() => usePackDrag(getNodes, setNodes))

    // Start drag to populate pack members
    act(() => {
      result.current.onNodeDragStart(mockEvent, rfNodes[0]!)
    })

    // Stop drag — persists dragged node + pack members
    const stoppedNode = { ...rfNodes[0]!, position: { x: 150, y: 130 } }
    act(() => {
      result.current.onNodeDragStop(mockEvent, stoppedNode)
    })

    // Dragged node persisted
    expect(mockUpdateStep).toHaveBeenCalledWith('proto-1', { position_x: 150, position_y: 130 })
    // Pack members persisted
    expect(mockUpdateStep).toHaveBeenCalledWith('doc-1', { position_x: 250, position_y: 80 })
    expect(mockUpdateStep).toHaveBeenCalledWith('ctx-1', { position_x: 350, position_y: 80 })
    expect(mockUpdateStep).toHaveBeenCalledTimes(3)
  })

  it('persists only the dragged node on solo drag stop', () => {
    const rfNodes: Node[] = [
      makeRFNode('ctx-1', 'contextNode', 200, 100, 'proto-1'),
    ]
    const getNodes = vi.fn<() => Node[]>(() => rfNodes)
    const setNodes = vi.fn()
    const mockEvent = {} as React.MouseEvent

    const { result } = renderHook(() => usePackDrag(getNodes, setNodes))

    // Start drag on context (solo)
    act(() => {
      result.current.onNodeDragStart(mockEvent, rfNodes[0]!)
    })

    // Stop drag
    const stoppedNode = { ...rfNodes[0]!, position: { x: 250, y: 130 } }
    act(() => {
      result.current.onNodeDragStop(mockEvent, stoppedNode)
    })

    expect(mockUpdateStep).toHaveBeenCalledWith('ctx-1', { position_x: 250, position_y: 130 })
    expect(mockUpdateStep).toHaveBeenCalledTimes(1)
  })

  it('skips persistence for doc-artifact nodes', () => {
    const rfNodes: Node[] = [
      makeRFNode('doc-artifact-123', 'documentNode', 200, 50, 'proto-1'),
    ]
    const getNodes = vi.fn<() => Node[]>(() => rfNodes)
    const setNodes = vi.fn()
    const mockEvent = {} as React.MouseEvent

    const { result } = renderHook(() => usePackDrag(getNodes, setNodes))

    act(() => {
      result.current.onNodeDragStart(mockEvent, rfNodes[0]!)
    })

    act(() => {
      result.current.onNodeDragStop(mockEvent, rfNodes[0]!)
    })

    expect(mockUpdateStep).not.toHaveBeenCalled()
  })
})
