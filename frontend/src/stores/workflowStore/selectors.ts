import { toArray, nmGet } from '../lib'
import type { WorkflowState } from './types'
import type { Workflow, WorkflowStep, WorkflowStepEdge, DocumentDef } from '@/types/workflow'
import type { Document } from '@/types/document'
import { STALE_THRESHOLD_MS } from './_store'

const EMPTY_DOCS: Document[] = []
const EMPTY_DEFS: DocumentDef[] = []

const selectAll = (s: WorkflowState): Workflow[] => toArray(s.items)

const selectById =
  (id: string) =>
  (s: WorkflowState): Workflow | undefined =>
    nmGet(s.items, id)

const selectActiveWorkflowId = (s: WorkflowState): string | null => s.activeWorkflowId

const selectSteps = (s: WorkflowState): WorkflowStep[] => toArray(s.steps)

const selectEdges = (s: WorkflowState): WorkflowStepEdge[] => toArray(s.edges)

const selectStepById =
  (id: string | null) =>
  (s: WorkflowState): WorkflowStep | null =>
    id !== null ? (nmGet(s.steps, id) ?? null) : null

const selectEdgeById =
  (id: string | null) =>
  (s: WorkflowState): WorkflowStepEdge | null =>
    id !== null ? (nmGet(s.edges, id) ?? null) : null

const selectStepDocuments =
  (stepId: string) =>
  (s: WorkflowState): Document[] =>
    s.documentsByStep[stepId] ?? EMPTY_DOCS

const selectStepDocumentDefs =
  (stepId: string) =>
  (s: WorkflowState): DocumentDef[] =>
    s.documentDefsByStep[stepId] ?? EMPTY_DEFS

const selectDocumentDefsByStep = (s: WorkflowState): Record<string, DocumentDef[]> =>
  s.documentDefsByStep

const selectLoading = (s: WorkflowState): boolean => s.loading

const selectError = (s: WorkflowState): string | null => s.error

const selectDirty = (s: WorkflowState): boolean => s.dirty

const selectDirtyStepIds = (s: WorkflowState): Set<string> => s.dirtyStepIds

const selectIsStale = (s: WorkflowState): boolean => s.lastFetched === null || Date.now() - s.lastFetched > STALE_THRESHOLD_MS

export {
  selectAll,
  selectById,
  selectActiveWorkflowId,
  selectSteps,
  selectEdges,
  selectStepById,
  selectEdgeById,
  selectStepDocuments,
  selectStepDocumentDefs,
  selectDocumentDefsByStep,
  selectLoading,
  selectError,
  selectDirty,
  selectDirtyStepIds,
  selectIsStale,
}
