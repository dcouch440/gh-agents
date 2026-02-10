import { describe, it, expect, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useProtocolHighlight, CanvasNodeKind, HighlightMode } from './useProtocolHighlight'
import { canvasStore } from '@/stores'

describe('useProtocolHighlight', () => {
  beforeEach(() => {
    canvasStore.reset()
  })

  describe('null protocolStepId', () => {
    it('returns none for context kind', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', null))
      expect(result.current).toBe(HighlightMode.NONE)
    })

    it('returns none for document kind', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', null))
      expect(result.current).toBe(HighlightMode.NONE)
    })

    it('returns none for step kind', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.STEP, 'step-1', null))
      expect(result.current).toBe(HighlightMode.NONE)
    })
  })

  describe('selected state', () => {
    it('returns select for context kind when protocol step is selected', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.selectSteps(['proto-1'])
      })
      expect(result.current).toBe(HighlightMode.SELECT)
    })

    it('returns select for document kind when protocol step is selected', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'proto-1'))
      act(() => {
        canvasStore.selectSteps(['proto-1'])
      })
      expect(result.current).toBe(HighlightMode.SELECT)
    })

    it('returns select for step kind when protocol step is selected', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.STEP, 'step-1', 'proto-1'))
      act(() => {
        canvasStore.selectSteps(['proto-1'])
      })
      expect(result.current).toBe(HighlightMode.SELECT)
    })

    it('returns select when protocol step is in highlightedProtocolStepIds', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.setHighlightedProtocols(new Set(['proto-1']))
      })
      expect(result.current).toBe(HighlightMode.SELECT)
    })

    it('returns none when a different protocol is highlighted', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.setHighlightedProtocols(new Set(['proto-2']))
      })
      expect(result.current).toBe(HighlightMode.NONE)
    })

    it('prioritizes select over hover', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.selectSteps(['proto-1'])
        canvasStore.setHoveredStep('proto-1')
      })
      expect(result.current).toBe(HighlightMode.SELECT)
    })
  })

  describe('protocol group hover — hovering the protocol node', () => {
    it('returns hover for context kind when protocol is hovered', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('proto-1', 'proto-1')
      })
      expect(result.current).toBe(HighlightMode.HOVER)
    })

    it('returns hover for document kind when protocol is hovered', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('proto-1', 'proto-1')
      })
      expect(result.current).toBe(HighlightMode.HOVER)
    })

    it('returns none for step kind when protocol is hovered', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.STEP, 'step-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('proto-1', 'proto-1')
      })
      expect(result.current).toBe(HighlightMode.NONE)
    })

    it('returns none when a different protocol is hovered', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('other-step', 'proto-2')
      })
      expect(result.current).toBe(HighlightMode.NONE)
    })

    it('transitions back to none when hover clears', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('proto-1', 'proto-1')
      })
      expect(result.current).toBe(HighlightMode.HOVER)
      act(() => {
        canvasStore.setHoveredStep(null)
      })
      expect(result.current).toBe(HighlightMode.NONE)
    })
  })

  describe('self-hover — hovering an individual group member', () => {
    it('returns hover for document node when it is directly hovered', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('doc-1')
      })
      expect(result.current).toBe(HighlightMode.HOVER)
    })

    it('returns hover for context node when it is directly hovered', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('ctx-1')
      })
      expect(result.current).toBe(HighlightMode.HOVER)
    })

    it('returns none for step node when it is directly hovered', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.STEP, 'step-1', 'proto-1'))
      act(() => {
        canvasStore.setHoveredStep('step-1')
      })
      expect(result.current).toBe(HighlightMode.NONE)
    })

    it('only the hovered document highlights, not its sibling', () => {
      const { result: doc1 } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'documenter-1'),
      )
      const { result: doc2 } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-2', 'documenter-1'),
      )

      act(() => {
        canvasStore.setHoveredStep('doc-1')
      })

      expect(doc1.current).toBe(HighlightMode.HOVER)
      expect(doc2.current).toBe(HighlightMode.NONE)
    })

    it('only the hovered document highlights, context sibling stays none', () => {
      const { result: doc } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'documenter-1'),
      )
      const { result: ctx } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'documenter-1'),
      )

      act(() => {
        canvasStore.setHoveredStep('doc-1')
      })

      expect(doc.current).toBe(HighlightMode.HOVER)
      expect(ctx.current).toBe(HighlightMode.NONE)
    })
  })

  describe('protocol not hovered or selected', () => {
    it('returns none for context kind', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'proto-1'))
      expect(result.current).toBe(HighlightMode.NONE)
    })

    it('returns none for step kind', () => {
      const { result } = renderHook(() => useProtocolHighlight(CanvasNodeKind.STEP, 'step-1', 'proto-1'))
      expect(result.current).toBe(HighlightMode.NONE)
    })
  })

  describe('multi-node protocol scenarios', () => {
    it('two document nodes with same protocolStepId both hover when protocol is hovered', () => {
      const { result: doc1 } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'documenter-1'),
      )
      const { result: doc2 } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-2', 'documenter-1'),
      )

      act(() => {
        canvasStore.setHoveredStep('documenter-1', 'documenter-1')
      })

      expect(doc1.current).toBe(HighlightMode.HOVER)
      expect(doc2.current).toBe(HighlightMode.HOVER)
    })

    it('two document nodes with same protocolStepId both select when protocol is selected', () => {
      const { result: doc1 } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'documenter-1'),
      )
      const { result: doc2 } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-2', 'documenter-1'),
      )

      act(() => {
        canvasStore.selectSteps(['documenter-1'])
      })

      expect(doc1.current).toBe(HighlightMode.SELECT)
      expect(doc2.current).toBe(HighlightMode.SELECT)
    })

    it('context node does not select when a different protocol is selected', () => {
      const { result: context } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'documenter-A'),
      )
      const { result: doc } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'documenter-B'),
      )

      act(() => {
        canvasStore.selectSteps(['documenter-B'])
      })

      expect(doc.current).toBe(HighlightMode.SELECT)
      expect(context.current).toBe(HighlightMode.NONE)
    })

    it('context node does not hover when a different protocol is hovered', () => {
      const { result: context } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.CONTEXT, 'ctx-1', 'documenter-A'),
      )
      const { result: doc } = renderHook(() =>
        useProtocolHighlight(CanvasNodeKind.DOCUMENT, 'doc-1', 'documenter-B'),
      )

      act(() => {
        canvasStore.setHoveredStep('documenter-B', 'documenter-B')
      })

      expect(doc.current).toBe(HighlightMode.HOVER)
      expect(context.current).toBe(HighlightMode.NONE)
    })
  })
})
