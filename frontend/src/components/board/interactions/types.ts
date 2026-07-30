// ============================================================================
// Shared Interaction Types
// ============================================================================

import type { ArrowElement, BoardElements, BoxElement, InteractionMode, SelectionState } from '../elements'

type SetElements = (fn: (s: BoardElements) => BoardElements) => void
type SetSelection = (fn: (s: SelectionState) => SelectionState) => void
type SetInteraction = (mode: InteractionMode) => void

type CanvasChange =
  | { readonly kind: 'moved'; readonly elementId: string; readonly x: number; readonly y: number; readonly width: number; readonly height: number }
  | { readonly kind: 'text_changed'; readonly elementId: string; readonly text: string; readonly width: number; readonly height: number }
  | { readonly kind: 'node_created'; readonly box: BoxElement }
  | { readonly kind: 'edge_created'; readonly arrow: ArrowElement }
  | { readonly kind: 'elements_deleted'; readonly deletedIds: ReadonlySet<string>; readonly elements: BoardElements }

type CanvasChangeCallback = (change: CanvasChange) => void

export type { CanvasChange, CanvasChangeCallback, SetElements, SetInteraction, SetSelection }
