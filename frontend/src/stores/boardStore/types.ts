import type { BoardSubmitResponse } from '@/types/board'

type SubmitStatus = 'idle' | 'submitting' | 'success' | 'error'

type BoardState = {
  /** Current submit lifecycle status. */
  readonly status: SubmitStatus
  /** Error message from the last failed submit, null otherwise. */
  readonly error: string | null
  /** Full response from the last successful submit, null before first submit. */
  readonly lastResponse: BoardSubmitResponse | null
  /** Whether the active workflow has never been submitted before. */
  readonly isFirstSubmit: boolean
  /** Excalidraw element_id to workflow step_id. Accumulated across submits. */
  readonly elementStepMap: Readonly<Record<string, string>>
  /** Excalidraw element_id to workflow edge_id. Accumulated across submits. */
  readonly elementEdgeMap: Readonly<Record<string, string>>
}

export type { BoardState, SubmitStatus }
