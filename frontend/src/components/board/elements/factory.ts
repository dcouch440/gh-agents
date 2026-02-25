// ============================================================================
// Element Factories — Create New Board Elements
// ============================================================================

import { BOARD } from '../constants'
import type { AnchorPoint, ArrowElement, BoardElements, BoxElement } from './types'

const createBox = (x: number, y: number, text = ''): BoxElement => ({
  id: crypto.randomUUID(),
  type: 'box',
  x,
  y,
  width: BOARD.DEFAULT_BOX_WIDTH,
  height: BOARD.DEFAULT_BOX_HEIGHT,
  text,
})

const createArrow = (
  sourceBoxId: string,
  targetBoxId: string,
  sourceAnchor: AnchorPoint,
  targetAnchor: AnchorPoint,
): ArrowElement => ({
  id: crypto.randomUUID(),
  type: 'arrow',
  sourceBoxId,
  targetBoxId,
  sourceAnchor,
  targetAnchor,
})

const emptyBoard = (): BoardElements => ({
  boxes: new Map(),
  arrows: new Map(),
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
  sourceAnchor: AnchorPoint,
  targetAnchor: AnchorPoint,
): ArrowElement => ({
  id,
  type: 'arrow',
  sourceBoxId,
  targetBoxId,
  sourceAnchor,
  targetAnchor,
})

export { createArrow, createArrowFromSaved, createBox, createBoxFromSaved, emptyBoard }
