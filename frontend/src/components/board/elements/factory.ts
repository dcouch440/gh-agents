// ============================================================================
// Element Factories — Create New Board Elements
// ============================================================================

import type { Point } from '@/utils/geometry'
import { BOARD } from '../constants'
import type { ArrowElement, BoardElements, BoxElement, FocusPoint, PenElement } from './types'

const createBox = (x: number, y: number, text = ''): BoxElement => ({
  id: crypto.randomUUID(),
  type: 'box',
  x,
  y,
  width: BOARD.DEFAULT_BOX_WIDTH,
  height: BOARD.DEFAULT_BOX_HEIGHT,
  text,
})

const createBoxWithSize = (x: number, y: number, width: number, height: number, text = ''): BoxElement => ({
  id: crypto.randomUUID(),
  type: 'box',
  x,
  y,
  width,
  height,
  text,
})

const createArrow = (
  sourceBoxId: string,
  targetBoxId: string,
  sourceFocus: FocusPoint,
  targetFocus: FocusPoint,
): ArrowElement => ({
  id: crypto.randomUUID(),
  type: 'arrow',
  sourceBoxId,
  targetBoxId,
  sourceFocus,
  targetFocus,
})

const createPen = (points: readonly Point[], pressures: readonly number[]): PenElement => ({
  id: crypto.randomUUID(),
  type: 'pen',
  points,
  pressures,
})

const emptyBoard = (): BoardElements => ({
  boxes: new Map(),
  arrows: new Map(),
  pens: new Map(),
  boxOrder: [],
})

// ── Reconstruct from saved data ────────────────────────────────────────────

const createBoxFromSaved = (
  id: string,
  x: number,
  y: number,
  width: number,
  height: number,
  text: string,
): BoxElement => ({
  id,
  type: 'box',
  x,
  y,
  width,
  height,
  text,
})

const createArrowFromSaved = (
  id: string,
  sourceBoxId: string,
  targetBoxId: string,
  sourceFocus: FocusPoint,
  targetFocus: FocusPoint,
): ArrowElement => ({
  id,
  type: 'arrow',
  sourceBoxId,
  targetBoxId,
  sourceFocus,
  targetFocus,
})

export { createArrow, createArrowFromSaved, createBox, createBoxFromSaved, createBoxWithSize, createPen, emptyBoard }
