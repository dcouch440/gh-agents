import { describe, it, expect, vi } from 'vitest'
import { render, screen, act } from '@/test/render'
import { Canvas2D } from './Canvas2D'
import { addBox, createBox, emptyBoard } from '../elements'
import type { BoardElements } from '../elements'
import type { DrawTheme } from './renderer'

// The board paints through Canvas 2D, which jsdom does not implement. Only the
// textarea overlay matters here, so stub the painter out.
vi.mock('./renderer', () => ({ renderBoard: vi.fn() }))

// ── Fixtures ─────────────────────────────────────────────────────────────

const BOX_ID = 'box-1'
const OTHER_ID = 'box-2'

const theme: DrawTheme = {
  canvasBg: '#000',
  gridDotColor: '#111',
  connectorColor: '#222',
  surfaceBg: '#333',
  accent: '#444',
  textColor: '#fff',
} as unknown as DrawTheme

const boardWith = (text: string, otherText = 'other'): BoardElements => {
  let board = addBox(emptyBoard(), { ...createBox(0, 0, text), id: BOX_ID })
  board = addBox(board, { ...createBox(300, 0, otherText), id: OTHER_ID })
  return board
}

const noop = () => {}

const renderCanvas = (elements: BoardElements, onBoxTextChange = vi.fn()) =>
  render(
    <Canvas2D
      ref={null}
      elements={elements}
      selection={{ selectedIds: new Set(), marquee: null }}
      editingBoxId={BOX_ID}
      activeTool="select"
      interaction={{ type: 'editing', boxId: BOX_ID }}
      viewport={{ panX: 0, panY: 0, zoom: 1 }}
      theme={theme}
      statusRings={new Map()}
      pulsing={false}
      previews={{ arrow: null, box: null, pen: null }}
      onPointerDown={noop}
      onPointerMove={noop}
      onPointerUp={noop}
      onWheel={noop}
      onDoubleClick={noop}
      onBoxTextChange={onBoxTextChange}
      onBoxDoubleClick={noop}
      onBoxBlur={noop}
      onBoxPointerDown={noop}
      onAnchorPointerDown={noop}
      onResizePointerDown={noop}
      onContextMenu={noop}
    />,
  )

const getOverlay = (): HTMLTextAreaElement => {
  const el = screen.getByRole('textbox')
  if (!(el instanceof HTMLTextAreaElement)) throw new Error('overlay is not a textarea')
  return el
}

// ── Tests ────────────────────────────────────────────────────────────────

describe('Canvas2D text overlay', () => {
  it('seeds the overlay with the box text and focuses it when editing starts', () => {
    renderCanvas(boardWith('hello'))
    const overlay = getOverlay()

    expect(overlay.value).toBe('hello')
    expect(document.activeElement).toBe(overlay)
  })

  it('keeps the caret where the user put it while typing mid-string', () => {
    const onBoxTextChange = vi.fn()
    const { rerender } = renderCanvas(boardWith('hello'), onBoxTextChange)
    const overlay = getOverlay()

    // Type an "X" between "hel" and "lo", as the browser would.
    overlay.value = 'helXlo'
    overlay.setSelectionRange(4, 4)
    act(() => {
      overlay.dispatchEvent(new Event('input', { bubbles: true }))
    })

    expect(onBoxTextChange).toHaveBeenCalledWith(BOX_ID, 'helXlo', expect.any(Number), expect.any(Number))

    // The store round-trips the new text back as a fresh elements object — the
    // exact shape that used to re-seed the value and slam the caret to the end.
    rerender(
      <Canvas2D
        ref={null}
        elements={boardWith('helXlo')}
        selection={{ selectedIds: new Set(), marquee: null }}
        editingBoxId={BOX_ID}
        activeTool="select"
        interaction={{ type: 'editing', boxId: BOX_ID }}
        viewport={{ panX: 0, panY: 0, zoom: 1 }}
        theme={theme}
        previews={{ arrow: null, box: null, pen: null }}
        onPointerDown={noop}
        onPointerMove={noop}
        onPointerUp={noop}
        onWheel={noop}
        onDoubleClick={noop}
        onBoxTextChange={onBoxTextChange}
        onBoxDoubleClick={noop}
        onBoxBlur={noop}
        onBoxPointerDown={noop}
        onAnchorPointerDown={noop}
        onResizePointerDown={noop}
        onContextMenu={noop}
      />,
    )

    expect(overlay.selectionStart).toBe(4)
    expect(overlay.selectionEnd).toBe(4)
    expect(overlay.value).toBe('helXlo')
  })

  it('does not disturb the overlay when a different box changes mid-edit', () => {
    const { rerender } = renderCanvas(boardWith('hello'))
    const overlay = getOverlay()

    overlay.value = 'hello world'
    overlay.setSelectionRange(2, 2)

    // A BOARD_ELEMENTS_UPDATED broadcast replaces the whole element set.
    rerender(
      <Canvas2D
        ref={null}
        elements={boardWith('hello', 'renamed by the agent')}
        selection={{ selectedIds: new Set(), marquee: null }}
        editingBoxId={BOX_ID}
        activeTool="select"
        interaction={{ type: 'editing', boxId: BOX_ID }}
        viewport={{ panX: 0, panY: 0, zoom: 1 }}
        theme={theme}
        previews={{ arrow: null, box: null, pen: null }}
        onPointerDown={noop}
        onPointerMove={noop}
        onPointerUp={noop}
        onWheel={noop}
        onDoubleClick={noop}
        onBoxTextChange={noop}
        onBoxDoubleClick={noop}
        onBoxBlur={noop}
        onBoxPointerDown={noop}
        onAnchorPointerDown={noop}
        onResizePointerDown={noop}
        onContextMenu={noop}
      />,
    )

    // The user's in-progress text survives, and so does the caret.
    expect(overlay.value).toBe('hello world')
    expect(overlay.selectionStart).toBe(2)
  })
})
