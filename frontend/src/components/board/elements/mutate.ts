// ============================================================================
// Mutate — Pure State Transitions for Board Elements
// ============================================================================
//
// Every function accepts a BoardElements and returns a new one. No side effects.
// Map/Set are rebuilt via new Map(existing) to preserve immutability.

import type { ArrowElement, BoardElements, BoxElement, PenElement } from './types'

// ── Box mutations ──────────────────────────────────────────────────────────

const addBox = (state: BoardElements, box: BoxElement): BoardElements => {
  const boxes = new Map(state.boxes)
  boxes.set(box.id, box)
  return { ...state, boxes, boxOrder: [...state.boxOrder, box.id] }
}

/**
 * Remove a box and all arrows connected to it (cascade delete).
 */
const removeBox = (state: BoardElements, boxId: string): BoardElements => {
  const boxes = new Map(state.boxes)
  boxes.delete(boxId)

  const arrows = new Map(state.arrows)
  for (const [arrowId, arrow] of state.arrows) {
    if (arrow.sourceBoxId === boxId || arrow.targetBoxId === boxId) {
      arrows.delete(arrowId)
    }
  }

  const boxOrder = state.boxOrder.filter((id) => id !== boxId)
  return { ...state, boxes, arrows, boxOrder }
}

const updateBoxPosition = (state: BoardElements, boxId: string, x: number, y: number): BoardElements => {
  const existing = state.boxes.get(boxId)
  if (existing === undefined) return state

  const boxes = new Map(state.boxes)
  boxes.set(boxId, { ...existing, x, y })
  return { ...state, boxes }
}

const updateBoxText = (
  state: BoardElements,
  boxId: string,
  text: string,
  width: number,
  height: number,
): BoardElements => {
  const existing = state.boxes.get(boxId)
  if (existing === undefined) return state

  const boxes = new Map(state.boxes)
  boxes.set(boxId, { ...existing, text, width, height })
  return { ...state, boxes }
}

const updateBoxSize = (state: BoardElements, boxId: string, width: number, height: number): BoardElements => {
  const existing = state.boxes.get(boxId)
  if (existing === undefined) return state

  const boxes = new Map(state.boxes)
  boxes.set(boxId, { ...existing, width, height })
  return { ...state, boxes }
}

/** Move a box to the front of the z-order. */
const bringToFront = (state: BoardElements, boxId: string): BoardElements => {
  const idx = state.boxOrder.indexOf(boxId)
  if (idx === -1 || idx === state.boxOrder.length - 1) return state

  const boxOrder = state.boxOrder.filter((id) => id !== boxId)
  boxOrder.push(boxId)
  return { ...state, boxOrder }
}

// ── Pen mutations ─────────────────────────────────────────────────────────

const addPen = (state: BoardElements, pen: PenElement): BoardElements => {
  const pens = new Map(state.pens)
  pens.set(pen.id, pen)
  return { ...state, pens }
}

// ── Arrow queries ─────────────────────────────────────────────────────────

/** Returns true if an arrow already exists from sourceBoxId → targetBoxId. */
const hasArrow = (state: BoardElements, sourceBoxId: string, targetBoxId: string): boolean => {
  for (const [, arrow] of state.arrows) {
    if (arrow.sourceBoxId === sourceBoxId && arrow.targetBoxId === targetBoxId) return true
  }
  return false
}

// ── Arrow mutations ────────────────────────────────────────────────────────

const addArrow = (state: BoardElements, arrow: ArrowElement): BoardElements => {
  const arrows = new Map(state.arrows)
  arrows.set(arrow.id, arrow)
  return { ...state, arrows }
}

const removeArrow = (state: BoardElements, arrowId: string): BoardElements => {
  const arrows = new Map(state.arrows)
  arrows.delete(arrowId)
  return { ...state, arrows }
}

// ── Mixed mutations ────────────────────────────────────────────────────────

/**
 * Remove multiple elements by ID. Handles both boxes and arrows.
 * Removing a box cascades to its connected arrows.
 */
const removeElements = (state: BoardElements, ids: ReadonlySet<string>): BoardElements => {
  let result = state

  // Remove boxes first (cascades arrows)
  for (const id of ids) {
    if (result.boxes.has(id)) {
      result = removeBox(result, id)
    }
  }

  // Remove any explicitly selected arrows that survived cascade
  for (const id of ids) {
    if (result.arrows.has(id)) {
      result = removeArrow(result, id)
    }
  }

  // Remove pens
  let pensChanged = false
  const pens = new Map(result.pens)
  for (const id of ids) {
    if (pens.has(id)) {
      pens.delete(id)
      pensChanged = true
    }
  }
  if (pensChanged) {
    result = { ...result, pens }
  }

  return result
}

export {
  addArrow,
  addBox,
  addPen,
  bringToFront,
  hasArrow,
  removeBox,
  removeElements,
  updateBoxPosition,
  updateBoxSize,
  updateBoxText,
}
