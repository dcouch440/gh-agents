import { describe, expect, it } from 'vitest'
import { createArrow, createBox, emptyBoard } from './factory'
import { addArrow, addBox, hasArrow } from './mutate'
import type { BoardElements } from './types'

// ============================================================================
// Helpers
// ============================================================================

const boardWith = (...fns: ((b: BoardElements) => BoardElements)[]): BoardElements => {
  let board = emptyBoard()
  for (const fn of fns) board = fn(board)
  return board
}

const boxA = createBox(0, 0, 'A')
const boxB = createBox(200, 0, 'B')
const boxC = createBox(400, 0, 'C')
const anchor = { side: 'right' as const, ratio: 0.5 }

// ============================================================================
// hasArrow
// ============================================================================

describe('hasArrow', () => {
  it('returns false on empty board', () => {
    expect(hasArrow(emptyBoard(), 'any', 'other')).toBe(false)
  })

  it('returns false when no matching arrow exists', () => {
    const board = boardWith(
      (b) => addBox(b, boxA),
      (b) => addBox(b, boxB),
    )
    expect(hasArrow(board, boxA.id, boxB.id)).toBe(false)
  })

  it('returns true when exact source→target arrow exists', () => {
    const arrow = createArrow(boxA.id, boxB.id, anchor, anchor)
    const board = boardWith(
      (b) => addBox(b, boxA),
      (b) => addBox(b, boxB),
      (b) => addArrow(b, arrow),
    )
    expect(hasArrow(board, boxA.id, boxB.id)).toBe(true)
  })

  it('returns false for reverse direction', () => {
    const arrow = createArrow(boxA.id, boxB.id, anchor, anchor)
    const board = boardWith(
      (b) => addBox(b, boxA),
      (b) => addBox(b, boxB),
      (b) => addArrow(b, arrow),
    )
    expect(hasArrow(board, boxB.id, boxA.id)).toBe(false)
  })

  it('returns true when multiple arrows exist and one matches', () => {
    const arrowAB = createArrow(boxA.id, boxB.id, anchor, anchor)
    const arrowAC = createArrow(boxA.id, boxC.id, anchor, anchor)
    const board = boardWith(
      (b) => addBox(b, boxA),
      (b) => addBox(b, boxB),
      (b) => addBox(b, boxC),
      (b) => addArrow(b, arrowAB),
      (b) => addArrow(b, arrowAC),
    )
    expect(hasArrow(board, boxA.id, boxC.id)).toBe(true)
  })
})
