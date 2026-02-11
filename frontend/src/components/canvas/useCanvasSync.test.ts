import { describe, it, expect, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import type { Node, Edge } from '@xyflow/react'
import { useCanvasSync, stylesEqual } from './useCanvasSync'

// ── Helpers ────────────────────────────────────────────────────────────

const makeNode = (id: string, overrides: Partial<Node> = {}): Node => ({
  id,
  position: { x: 0, y: 0 },
  data: { label: id },
  type: 'step',
  ...overrides,
})

const makeEdge = (id: string, source: string, target: string, overrides: Partial<Edge> = {}): Edge => ({
  id,
  source,
  target,
  type: 'step',
  ...overrides,
})

/**
 * Renders the hook and returns the last updater function passed to setNodes/setEdges.
 * Since the hook calls set*(updater) inside useEffect, we capture the updater from the mock.
 */
const renderSync = (rfNodes: Node[], rfEdges: Edge[]) => {
  const setNodes = vi.fn()
  const setEdges = vi.fn()
  renderHook(() => useCanvasSync(rfNodes, rfEdges, setNodes, setEdges))
  return { setNodes, setEdges }
}

const lastUpdater = <T,>(mockFn: ReturnType<typeof vi.fn>): ((current: T[]) => T[]) => {
  const calls = mockFn.mock.calls
  const lastCall = calls[calls.length - 1]
  return lastCall[0] as (current: T[]) => T[]
}

// ── stylesEqual ────────────────────────────────────────────────────────

describe('stylesEqual', () => {
  it('returns true for identical references', () => {
    const s = { width: 100, height: 50 }
    expect(stylesEqual(s, s)).toBe(true)
  })

  it('returns true when both are undefined', () => {
    expect(stylesEqual(undefined, undefined)).toBe(true)
  })

  it('returns false when only first is undefined', () => {
    expect(stylesEqual(undefined, { width: 10 })).toBe(false)
  })

  it('returns false when only second is undefined', () => {
    expect(stylesEqual({ width: 10 }, undefined)).toBe(false)
  })

  it('returns true when width and height match', () => {
    expect(stylesEqual({ width: 100, height: 50 }, { width: 100, height: 50 })).toBe(true)
  })

  it('returns false when width differs', () => {
    expect(stylesEqual({ width: 100, height: 50 }, { width: 200, height: 50 })).toBe(false)
  })

  it('returns false when height differs', () => {
    expect(stylesEqual({ width: 100, height: 50 }, { width: 100, height: 99 })).toBe(false)
  })
})

// ── Node sync ──────────────────────────────────────────────────────────

describe('useCanvasSync — node sync', () => {
  describe('structural changes', () => {
    it('adds new nodes while preserving existing selection and position', () => {
      const nodeA = makeNode('a')
      const nodeB = makeNode('b', { position: { x: 50, y: 50 } })
      const { setNodes } = renderSync([nodeA, nodeB], [])
      const updater = lastUpdater<Node>(setNodes)

      const current = [
        { ...nodeA, position: { x: 10, y: 20 }, selected: true },
      ] as Node[]

      const result = updater(current)
      expect(result).toHaveLength(2)
      // Existing node preserves position and selection
      expect(result[0]!.id).toBe('a')
      expect(result[0]!.position).toEqual({ x: 10, y: 20 })
      expect(result[0]!.selected).toBe(true)
      // New node gets its computed position, unselected
      expect(result[1]!.id).toBe('b')
      expect(result[1]!.position).toEqual({ x: 50, y: 50 })
      expect(result[1]!.selected).toBe(false)
    })

    it('removes nodes that are no longer in rfNodes', () => {
      const nodeA = makeNode('a')
      const { setNodes } = renderSync([nodeA], [])
      const updater = lastUpdater<Node>(setNodes)

      const current = [
        { ...nodeA, selected: false },
        makeNode('b'),
      ] as Node[]

      const result = updater(current)
      expect(result).toHaveLength(1)
      expect(result[0]!.id).toBe('a')
    })

    it('defaults selection to false for nodes not in current', () => {
      const nodeA = makeNode('a')
      const { setNodes } = renderSync([nodeA], [])
      const updater = lastUpdater<Node>(setNodes)

      const result = updater([])
      expect(result).toHaveLength(1)
      expect(result[0]!.selected).toBe(false)
    })
  })

  describe('data-only changes', () => {
    it('updates node data without touching position or selection', () => {
      const nodeA = makeNode('a', { data: { label: 'updated' } })
      const { setNodes } = renderSync([nodeA], [])
      const updater = lastUpdater<Node>(setNodes)

      const current = [
        makeNode('a', {
          data: { label: 'old' },
          position: { x: 99, y: 88 },
          selected: true,
        }),
      ] as Node[]

      const result = updater(current)
      expect(result).not.toBe(current) // new array
      expect(result[0]!.data).toEqual({ label: 'updated' })
      expect(result[0]!.position).toEqual({ x: 99, y: 88 })
      expect(result[0]!.selected).toBe(true)
    })

    it('updates node type without touching position', () => {
      const nodeA = makeNode('a', { type: 'context' })
      const { setNodes } = renderSync([nodeA], [])
      const updater = lastUpdater<Node>(setNodes)

      const current = [
        makeNode('a', { type: 'step', position: { x: 5, y: 5 } }),
      ] as Node[]

      const result = updater(current)
      expect(result[0]!.type).toBe('context')
      expect(result[0]!.position).toEqual({ x: 5, y: 5 })
    })

    it('updates node style when dimensions change', () => {
      const nodeA = makeNode('a', { style: { width: 200, height: 100 } })
      const { setNodes } = renderSync([nodeA], [])
      const updater = lastUpdater<Node>(setNodes)

      const current = [
        makeNode('a', { style: { width: 150, height: 100 } }),
      ] as Node[]

      const result = updater(current)
      expect(result[0]!.style).toEqual({ width: 200, height: 100 })
    })

    it('returns same reference when nothing changed', () => {
      const data = { label: 'same' }
      const nodeA = makeNode('a', { data, type: 'step', style: { width: 10 } })
      const { setNodes } = renderSync([nodeA], [])
      const updater = lastUpdater<Node>(setNodes)

      // Current has same data object reference, same type, same style dimensions
      const current = [
        makeNode('a', { data, type: 'step', style: { width: 10 } }),
      ] as Node[]

      const result = updater(current)
      expect(result).toBe(current)
    })

    it('preserves unchanged nodes by reference in mixed updates', () => {
      const sharedData = { label: 'unchanged' }
      const nodeA = makeNode('a', { data: sharedData })
      const nodeB = makeNode('b', { data: { label: 'new-b' } })
      const { setNodes } = renderSync([nodeA, nodeB], [])
      const updater = lastUpdater<Node>(setNodes)

      const currentA = makeNode('a', { data: sharedData })
      const currentB = makeNode('b', { data: { label: 'old-b' } })
      const current = [currentA, currentB]

      const result = updater(current)
      expect(result).not.toBe(current) // new array due to B change
      expect(result[0]).toBe(currentA) // A preserved by reference
      expect(result[1]).not.toBe(currentB) // B is a new object
      expect(result[1]!.data).toEqual({ label: 'new-b' })
    })
  })
})

// ── Edge sync ──────────────────────────────────────────────────────────

describe('useCanvasSync — edge sync', () => {
  describe('structural changes', () => {
    it('adds new edges while preserving selection on existing ones', () => {
      const edgeAB = makeEdge('e1', 'a', 'b')
      const edgeBC = makeEdge('e2', 'b', 'c')
      const { setEdges } = renderSync([], [edgeAB, edgeBC])
      const updater = lastUpdater<Edge>(setEdges)

      const current = [
        { ...edgeAB, selected: true },
      ] as Edge[]

      const result = updater(current)
      expect(result).toHaveLength(2)
      expect(result[0]!.selected).toBe(true)
      expect(result[1]!.selected).toBe(false)
    })

    it('removes edges that are no longer in rfEdges', () => {
      const edgeAB = makeEdge('e1', 'a', 'b')
      const { setEdges } = renderSync([], [edgeAB])
      const updater = lastUpdater<Edge>(setEdges)

      const current = [
        edgeAB,
        makeEdge('e2', 'b', 'c'),
      ]

      const result = updater(current)
      expect(result).toHaveLength(1)
      expect(result[0]!.id).toBe('e1')
    })

    it('defaults selection to false for edges not in current', () => {
      const edgeAB = makeEdge('e1', 'a', 'b')
      const { setEdges } = renderSync([], [edgeAB])
      const updater = lastUpdater<Edge>(setEdges)

      const result = updater([])
      expect(result).toHaveLength(1)
      expect(result[0]!.selected).toBe(false)
    })
  })

  describe('data-only changes', () => {
    it('updates edge source', () => {
      const edgeAB = makeEdge('e1', 'x', 'b')
      const { setEdges } = renderSync([], [edgeAB])
      const updater = lastUpdater<Edge>(setEdges)

      const current = [makeEdge('e1', 'a', 'b')] as Edge[]

      const result = updater(current)
      expect(result[0]!.source).toBe('x')
    })

    it('updates edge target', () => {
      const edgeAB = makeEdge('e1', 'a', 'z')
      const { setEdges } = renderSync([], [edgeAB])
      const updater = lastUpdater<Edge>(setEdges)

      const current = [makeEdge('e1', 'a', 'b')] as Edge[]

      const result = updater(current)
      expect(result[0]!.target).toBe('z')
    })

    it('updates edge type', () => {
      const edgeAB = makeEdge('e1', 'a', 'b', { type: 'protocol' })
      const { setEdges } = renderSync([], [edgeAB])
      const updater = lastUpdater<Edge>(setEdges)

      const current = [makeEdge('e1', 'a', 'b', { type: 'step' })] as Edge[]

      const result = updater(current)
      expect(result[0]!.type).toBe('protocol')
    })

    it('returns same reference when nothing changed', () => {
      const edgeAB = makeEdge('e1', 'a', 'b')
      const { setEdges } = renderSync([], [edgeAB])
      const updater = lastUpdater<Edge>(setEdges)

      const current = [makeEdge('e1', 'a', 'b')] as Edge[]

      const result = updater(current)
      expect(result).toBe(current)
    })

    it('preserves unchanged edges by reference in mixed updates', () => {
      const edgeAB = makeEdge('e1', 'a', 'b')
      const edgeBC = makeEdge('e2', 'b', 'z') // changed target
      const { setEdges } = renderSync([], [edgeAB, edgeBC])
      const updater = lastUpdater<Edge>(setEdges)

      const currentE1 = makeEdge('e1', 'a', 'b')
      const currentE2 = makeEdge('e2', 'b', 'c')
      const current = [currentE1, currentE2]

      const result = updater(current)
      expect(result).not.toBe(current)
      expect(result[0]).toBe(currentE1) // unchanged, same ref
      expect(result[1]).not.toBe(currentE2) // changed
      expect(result[1]!.target).toBe('z')
    })
  })
})
