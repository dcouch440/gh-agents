import { store } from './_store'
import {
  selectStatus,
  selectError,
  selectLastResponse,
  selectIsFirstSubmit,
  selectElementStepMap,
  selectElementEdgeMap,
  selectIsSubmitting,
} from './selectors'
import { submitBoard, resetBoard } from './submit'

export const boardStore = {
  store,
  selectStatus,
  selectError,
  selectLastResponse,
  selectIsFirstSubmit,
  selectElementStepMap,
  selectElementEdgeMap,
  selectIsSubmitting,
  submitBoard,
  resetBoard,
}

export type { BoardState, SubmitStatus } from './types'
