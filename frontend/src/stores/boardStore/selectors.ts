import type { BoardState, SubmitStatus } from './types'
import type { BoardSubmitResponse } from '@/types/board'

const selectStatus = (s: BoardState): SubmitStatus => s.status
const selectError = (s: BoardState): string | null => s.error
const selectLastResponse = (s: BoardState): BoardSubmitResponse | null => s.lastResponse
const selectIsFirstSubmit = (s: BoardState): boolean => s.isFirstSubmit
const selectElementStepMap = (s: BoardState): Readonly<Record<string, string>> => s.elementStepMap
const selectElementEdgeMap = (s: BoardState): Readonly<Record<string, string>> => s.elementEdgeMap
const selectIsSubmitting = (s: BoardState): boolean => s.status === 'submitting'

export {
  selectStatus,
  selectError,
  selectLastResponse,
  selectIsFirstSubmit,
  selectElementStepMap,
  selectElementEdgeMap,
  selectIsSubmitting,
}
