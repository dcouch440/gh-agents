import { describe, expect, it } from 'vitest'
import { createArrow, createBox, createPen, emptyBoard } from './factory'
import { addArrow, addBox, addPen } from './mutate'
import { serializeToExcalidraw, textIdForBox } from './serialize'
import { deserializeFromExcalidraw } from './deserialize'
import type { BoardElements } from './types'

// ============================================================================
// Helpers
// ============================================================================

const boardWith = (...fns: ((b: BoardElements) => BoardElements)[]): BoardElements => {
  let board = emptyBoard()
  for (const fn of fns) board = fn(board)
  return board
}

// ============================================================================
// serializeToExcalidraw
// ============================================================================

describe('serializeToExcalidraw', () => {
  it('serializes an empty board to an empty array', () => {
    const result = serializeToExcalidraw(emptyBoard())
    expect(result).toEqual([])
  })

  it('serializes a single box into rectangle + text element pair', () => {
    const box = createBox(100, 200, 'Research competitors')
    const board = addBox(emptyBoard(), box)
    const result = serializeToExcalidraw(board)

    expect(result).toHaveLength(2)

    // Rectangle
    const rect = result[0]!
    expect(rect['type']).toBe('rectangle')
    expect(rect['id']).toBe(box.id)
    expect(rect['x']).toBe(100)
    expect(rect['y']).toBe(200)
    expect(rect['width']).toBe(box.width)
    expect(rect['height']).toBe(box.height)
    expect(rect['isDeleted']).toBe(false)

    // boundElements must include the text ref
    const bound = rect['boundElements'] as { id: string; type: string }[]
    expect(bound).toContainEqual({ id: textIdForBox(box.id), type: 'text' })

    // Text
    const text = result[1]!
    expect(text['type']).toBe('text')
    expect(text['id']).toBe(textIdForBox(box.id))
    expect(text['text']).toBe('Research competitors')
    expect(text['containerId']).toBe(box.id)
    expect(text['isDeleted']).toBe(false)
  })

  it('rectangle boundElements includes connected arrow refs', () => {
    const box1 = createBox(100, 100, 'Source')
    const box2 = createBox(400, 100, 'Target')
    const arrow = createArrow(box1.id, box2.id, { fx: 1, fy: 0.5 }, { fx: 0, fy: 0.5 })
    const board = boardWith(
      (b) => addBox(b, box1),
      (b) => addBox(b, box2),
      (b) => addArrow(b, arrow),
    )
    const result = serializeToExcalidraw(board)

    // Find the rectangle for box1
    const rect1 = result.find((el) => el['id'] === box1.id)!
    const bound1 = rect1['boundElements'] as { id: string; type: string }[]
    expect(bound1).toContainEqual({ id: arrow.id, type: 'arrow' })

    // Find the rectangle for box2
    const rect2 = result.find((el) => el['id'] === box2.id)!
    const bound2 = rect2['boundElements'] as { id: string; type: string }[]
    expect(bound2).toContainEqual({ id: arrow.id, type: 'arrow' })
  })

  it('serializes arrows with startBinding and endBinding', () => {
    const box1 = createBox(100, 100, 'A')
    const box2 = createBox(400, 100, 'B')
    const arrow = createArrow(box1.id, box2.id, { fx: 1, fy: 0.5 }, { fx: 0, fy: 0.5 })
    const board = boardWith(
      (b) => addBox(b, box1),
      (b) => addBox(b, box2),
      (b) => addArrow(b, arrow),
    )
    const result = serializeToExcalidraw(board)

    const arrowEl = result.find((el) => el['type'] === 'arrow')!
    expect(arrowEl['id']).toBe(arrow.id)
    expect(arrowEl['startBinding']).toEqual({ elementId: box1.id })
    expect(arrowEl['endBinding']).toEqual({ elementId: box2.id })
    expect(arrowEl['isDeleted']).toBe(false)
  })

  it('uses camelCase field names matching backend serde expectations', () => {
    const box = createBox(0, 0, 'test')
    const board = addBox(emptyBoard(), box)
    const result = serializeToExcalidraw(board)

    const rect = result[0]!
    // These must be camelCase — the backend uses #[serde(rename_all = "camelCase")]
    expect('isDeleted' in rect).toBe(true)
    expect('boundElements' in rect).toBe(true)
    // snake_case versions must NOT exist
    expect('is_deleted' in rect).toBe(false)
    expect('bound_elements' in rect).toBe(false)

    const text = result[1]!
    expect('containerId' in text).toBe(true)
    expect('container_id' in text).toBe(false)
  })

  it('text element position is offset by padding from rectangle', () => {
    const box = createBox(100, 200, 'hello')
    const board = addBox(emptyBoard(), box)
    const result = serializeToExcalidraw(board)

    const rect = result[0]!
    const text = result[1]!

    const paddingX = 20 // BOARD.BOX_PADDING_X
    const paddingY = 12 // BOARD.BOX_PADDING_Y

    expect(text['x']).toBe((rect['x'] as number) + paddingX)
    expect(text['y']).toBe((rect['y'] as number) + paddingY)
  })

  it('serializes pen strokes as freedraw elements with relative coordinates', () => {
    const pen = createPen(
      [{ x: 100, y: 200 }, { x: 110, y: 205 }, { x: 120, y: 200 }],
      [0.5, 0.7, 0.5],
    )
    const board = addPen(emptyBoard(), pen)
    const result = serializeToExcalidraw(board)

    expect(result).toHaveLength(1)
    const freedraw = result[0]!
    expect(freedraw['type']).toBe('freedraw')
    expect(freedraw['id']).toBe(pen.id)
    expect(freedraw['x']).toBe(100) // min x
    expect(freedraw['y']).toBe(200) // min y
    expect(freedraw['isDeleted']).toBe(false)

    // Points should be relative to (x, y)
    const points = freedraw['points'] as number[][]
    expect(points).toHaveLength(3)
    expect(points[0]).toEqual([0, 0, 0.5])
    expect(points[1]).toEqual([10, 5, 0.7])
    expect(points[2]).toEqual([20, 0, 0.5])
  })
})

// ============================================================================
// Round-trip: deserialize(serialize(board)) preserves data
// ============================================================================

describe('serialize → deserialize round-trip', () => {
  it('preserves a single box', () => {
    const box = createBox(150, 250, 'Write report')
    const original = addBox(emptyBoard(), box)
    const serialized = serializeToExcalidraw(original)
    const restored = deserializeFromExcalidraw(serialized)

    expect(restored.boxes.size).toBe(1)
    const restoredBox = restored.boxes.get(box.id)!
    expect(restoredBox.id).toBe(box.id)
    expect(restoredBox.text).toBe('Write report')
    expect(restoredBox.x).toBe(150)
    expect(restoredBox.y).toBe(250)
    expect(restoredBox.width).toBe(box.width)
    expect(restoredBox.height).toBe(box.height)
  })

  it('preserves multiple boxes and arrows', () => {
    const box1 = createBox(100, 100, 'Research')
    const box2 = createBox(400, 100, 'Write')
    const box3 = createBox(250, 300, 'Review')
    const arrow1 = createArrow(box1.id, box2.id, { fx: 1, fy: 0.5 }, { fx: 0, fy: 0.5 })
    const arrow2 = createArrow(box2.id, box3.id, { fx: 0.5, fy: 1 }, { fx: 0.5, fy: 0 })

    const original = boardWith(
      (b) => addBox(b, box1),
      (b) => addBox(b, box2),
      (b) => addBox(b, box3),
      (b) => addArrow(b, arrow1),
      (b) => addArrow(b, arrow2),
    )

    const serialized = serializeToExcalidraw(original)
    const restored = deserializeFromExcalidraw(serialized)

    expect(restored.boxes.size).toBe(3)
    expect(restored.arrows.size).toBe(2)
    expect(restored.boxOrder).toHaveLength(3)

    // Arrow connections preserved
    const a1 = restored.arrows.get(arrow1.id)!
    expect(a1.sourceBoxId).toBe(box1.id)
    expect(a1.targetBoxId).toBe(box2.id)

    const a2 = restored.arrows.get(arrow2.id)!
    expect(a2.sourceBoxId).toBe(box2.id)
    expect(a2.targetBoxId).toBe(box3.id)
  })

  it('preserves box IDs (critical for backend element_id tracking)', () => {
    const box = createBox(0, 0, 'test')
    const original = addBox(emptyBoard(), box)
    const serialized = serializeToExcalidraw(original)
    const restored = deserializeFromExcalidraw(serialized)

    expect(restored.boxes.has(box.id)).toBe(true)
  })

  it('round-trips pen strokes via freedraw format', () => {
    const pen = createPen(
      [{ x: 50, y: 60 }, { x: 70, y: 80 }, { x: 90, y: 60 }],
      [0.5, 0.8, 0.5],
    )
    const original = addPen(emptyBoard(), pen)
    const serialized = serializeToExcalidraw(original)
    const restored = deserializeFromExcalidraw(serialized)

    expect(restored.pens.size).toBe(1)
    const restoredPen = restored.pens.get(pen.id)!
    expect(restoredPen.id).toBe(pen.id)
    expect(restoredPen.points).toHaveLength(3)

    // Points should be reconstructed to absolute coordinates
    expect(restoredPen.points[0]!.x).toBeCloseTo(50)
    expect(restoredPen.points[0]!.y).toBeCloseTo(60)
    expect(restoredPen.points[2]!.x).toBeCloseTo(90)
    expect(restoredPen.points[2]!.y).toBeCloseTo(60)

    // Pressures should be preserved
    expect(restoredPen.pressures[1]).toBeCloseTo(0.8)
  })

  it('handles empty text boxes', () => {
    const box = createBox(0, 0, '')
    const original = addBox(emptyBoard(), box)
    const serialized = serializeToExcalidraw(original)

    // Backend skips rectangles with empty text, so deserialize should too
    const restored = deserializeFromExcalidraw(serialized)
    // An empty-text box still has a text element, so it should survive round-trip
    // The text is '' which the backend will classify as a node with empty raw_text
    expect(restored.boxes.size).toBe(1)
    expect(restored.boxes.get(box.id)!.text).toBe('')
  })
})

// ============================================================================
// deserializeFromExcalidraw
// ============================================================================

describe('deserializeFromExcalidraw', () => {
  it('returns empty board for empty array', () => {
    const result = deserializeFromExcalidraw([])
    expect(result.boxes.size).toBe(0)
    expect(result.arrows.size).toBe(0)
  })

  it('skips deleted elements', () => {
    const elements = [
      { type: 'rectangle', id: 'r1', x: 0, y: 0, width: 100, height: 50, isDeleted: true, boundElements: [{ id: 'r1-text', type: 'text' }] },
      { type: 'text', id: 'r1-text', x: 10, y: 10, width: 80, height: 30, isDeleted: false, text: 'hello', containerId: 'r1' },
    ]
    const result = deserializeFromExcalidraw(elements)
    expect(result.boxes.size).toBe(0)
  })

  it('skips rectangles without bound text', () => {
    const elements = [
      { type: 'rectangle', id: 'r1', x: 0, y: 0, width: 100, height: 50, isDeleted: false, boundElements: [] },
    ]
    const result = deserializeFromExcalidraw(elements)
    expect(result.boxes.size).toBe(0)
  })

  it('skips arrows with missing bindings', () => {
    const elements = [
      { type: 'rectangle', id: 'r1', x: 0, y: 0, width: 100, height: 50, isDeleted: false, boundElements: [{ id: 'r1-text', type: 'text' }] },
      { type: 'text', id: 'r1-text', x: 10, y: 10, width: 80, height: 30, isDeleted: false, text: 'hello', containerId: 'r1' },
      { type: 'arrow', id: 'a1', x: 0, y: 0, width: 0, height: 0, isDeleted: false, startBinding: { elementId: 'r1' }, endBinding: null },
    ]
    const result = deserializeFromExcalidraw(elements)
    expect(result.arrows.size).toBe(0)
  })

  it('skips arrows referencing unknown boxes', () => {
    const elements = [
      { type: 'rectangle', id: 'r1', x: 0, y: 0, width: 100, height: 50, isDeleted: false, boundElements: [{ id: 'r1-text', type: 'text' }] },
      { type: 'text', id: 'r1-text', x: 10, y: 10, width: 80, height: 30, isDeleted: false, text: 'hello', containerId: 'r1' },
      { type: 'arrow', id: 'a1', x: 0, y: 0, width: 0, height: 0, isDeleted: false, startBinding: { elementId: 'r1' }, endBinding: { elementId: 'unknown' } },
    ]
    const result = deserializeFromExcalidraw(elements)
    expect(result.arrows.size).toBe(0)
  })
})
