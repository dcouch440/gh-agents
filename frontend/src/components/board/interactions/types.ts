// ============================================================================
// Shared Interaction Types
// ============================================================================

import type { BoardElements, InteractionMode, SelectionState } from '../elements'

type SetElements = (fn: (s: BoardElements) => BoardElements) => void
type SetSelection = (fn: (s: SelectionState) => SelectionState) => void
type SetInteraction = (mode: InteractionMode) => void

export type { SetElements, SetInteraction, SetSelection }
