import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useKeyboard } from './useKeyboard'
import { addBox, createBox, emptyBoard } from '../elements'
import type { ActiveTool, BoardElements } from '../elements'
import type { CanvasChangeCallback, SetElements, SetInteraction, SetSelection } from './types'

// ── Harness ──────────────────────────────────────────────────────────────

const BOX_ID = 'box-1'

type Harness = {
  elements: BoardElements
  setElements: ReturnType<typeof vi.fn<SetElements>>
  setSelection: ReturnType<typeof vi.fn<SetSelection>>
  setInteraction: ReturnType<typeof vi.fn<SetInteraction>>
  onDelete: ReturnType<typeof vi.fn<(ids: ReadonlySet<string>) => void>>
  setActiveTool: ReturnType<typeof vi.fn<(tool: ActiveTool) => void>>
  onCanvasChange: ReturnType<typeof vi.fn<CanvasChangeCallback>>
}

const makeHarness = (): Harness => {
  const box = { ...createBox(0, 0, 'hello'), id: BOX_ID }
  return {
    elements: addBox(emptyBoard(), box),
    setElements: vi.fn<SetElements>(),
    setSelection: vi.fn<SetSelection>(),
    setInteraction: vi.fn<SetInteraction>(),
    onDelete: vi.fn<(ids: ReadonlySet<string>) => void>(),
    setActiveTool: vi.fn<(tool: ActiveTool) => void>(),
    onCanvasChange: vi.fn<CanvasChangeCallback>(),
  }
}

const mount = (h: Harness, selectedIds: ReadonlySet<string>) =>
  renderHook(() =>
    useKeyboard(
      h.elements,
      h.setElements,
      { selectedIds },
      h.setSelection,
      { type: 'idle' },
      h.setInteraction,
      h.onDelete,
      h.setActiveTool,
      h.onCanvasChange,
    ),
  )

/** Dispatch a keydown on `window` as the browser would, with a real target. */
const press = (key: string, target: EventTarget, init: KeyboardEventInit = {}): void => {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init })
  target.dispatchEvent(event)
}

// ── Tests ────────────────────────────────────────────────────────────────

describe('useKeyboard', () => {
  let textarea: HTMLTextAreaElement

  beforeEach(() => {
    textarea = document.createElement('textarea')
    document.body.appendChild(textarea)
  })

  afterEach(() => {
    textarea.remove()
  })

  describe('when the user is typing in a text field', () => {
    it('does not delete selected elements on Backspace', () => {
      const h = makeHarness()
      mount(h, new Set([BOX_ID]))

      press('Backspace', textarea)

      expect(h.setElements).not.toHaveBeenCalled()
      expect(h.onDelete).not.toHaveBeenCalled()
      expect(h.onCanvasChange).not.toHaveBeenCalled()
    })

    it('does not delete selected elements on Delete', () => {
      const h = makeHarness()
      mount(h, new Set([BOX_ID]))

      press('Delete', textarea)

      expect(h.onDelete).not.toHaveBeenCalled()
    })

    it.each(['v', 'b', 'a', 'p'])('does not switch tools when typing %s', (key) => {
      const h = makeHarness()
      mount(h, new Set())

      press(key, textarea)

      expect(h.setActiveTool).not.toHaveBeenCalled()
    })

    it('does not select all elements on Cmd+A', () => {
      const h = makeHarness()
      mount(h, new Set())

      press('a', textarea, { metaKey: true })

      expect(h.setSelection).not.toHaveBeenCalled()
    })

    it('does not clear the board selection on Escape', () => {
      const h = makeHarness()
      mount(h, new Set([BOX_ID]))

      press('Escape', textarea)

      expect(h.setSelection).not.toHaveBeenCalled()
      expect(h.setInteraction).not.toHaveBeenCalled()
    })

    it('ignores keys from a contenteditable composer too', () => {
      const div = document.createElement('div')
      Object.defineProperty(div, 'isContentEditable', { value: true })
      document.body.appendChild(div)
      const h = makeHarness()
      mount(h, new Set([BOX_ID]))

      press('Backspace', div)

      expect(h.onDelete).not.toHaveBeenCalled()
      div.remove()
    })
  })

  describe('when focus is on the board', () => {
    it('deletes selected elements on Backspace', () => {
      const h = makeHarness()
      mount(h, new Set([BOX_ID]))

      press('Backspace', document.body)

      expect(h.setElements).toHaveBeenCalled()
      expect(h.onDelete).toHaveBeenCalledWith(new Set([BOX_ID]))
      expect(h.onCanvasChange).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'elements_deleted' }),
      )
    })

    it('switches tools on bare letter keys', () => {
      const h = makeHarness()
      mount(h, new Set())

      press('b', document.body)

      expect(h.setActiveTool).toHaveBeenCalledWith('box')
    })

    it('clears the selection on Escape', () => {
      const h = makeHarness()
      mount(h, new Set([BOX_ID]))

      press('Escape', document.body)

      expect(h.setSelection).toHaveBeenCalled()
    })
  })

  describe('undo/redo', () => {
    it('does nothing on Cmd+Z — undo is no longer exposed', () => {
      const h = makeHarness()
      mount(h, new Set([BOX_ID]))

      press('z', document.body, { metaKey: true })

      expect(h.setElements).not.toHaveBeenCalled()
      expect(h.setSelection).not.toHaveBeenCalled()
    })
  })
})
