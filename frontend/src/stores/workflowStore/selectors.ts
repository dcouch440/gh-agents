import { toArray, nmGet } from '../lib'
import type { WorkflowState } from './types'
import type { Workflow, WorkflowStep, WorkflowStepEdge, DocumentDef, RosterAgent, RoomStepMember } from '@/types/workflow'
import type { Document } from '@/types/document'
import type { ConsistencyIssue } from '@/types/ws'
import { STALE_THRESHOLD_MS } from './_store'

const EMPTY_DOCS: Document[] = []
const EMPTY_DEFS: DocumentDef[] = []
const EMPTY_ROSTER: RosterAgent[] = []
const EMPTY_ROOM_MEMBERS: RoomStepMember[] = []
const EMPTY_ISSUES: ConsistencyIssue[] = []

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

const selectStepRoster =
  (stepId: string) =>
  (s: WorkflowState): RosterAgent[] =>
    s.rosterByStep[stepId] ?? EMPTY_ROSTER

const selectRosterByStep = (s: WorkflowState): Record<string, RosterAgent[]> =>
  s.rosterByStep

const selectRoomStepMembers =
  (stepId: string) =>
  (s: WorkflowState): RoomStepMember[] =>
    s.roomMembersByStep[stepId] ?? EMPTY_ROOM_MEMBERS

const selectRoomMembersByStep = (s: WorkflowState): Record<string, RoomStepMember[]> =>
  s.roomMembersByStep

const selectNotesByStep = (s: WorkflowState): Record<string, string> =>
  s.notesByStep

const selectDocumentContentByDefId = (s: WorkflowState): Record<string, string> =>
  s.documentContentByDefId

const selectLoading = (s: WorkflowState): boolean => s.loading

const selectError = (s: WorkflowState): string | null => s.error

const selectDirty = (s: WorkflowState): boolean => s.dirty

const selectDirtyStepIds = (s: WorkflowState): Set<string> => s.dirtyStepIds

const selectIssuesByStep = (s: WorkflowState): Record<string, ConsistencyIssue[]> =>
  s.issuesByStep

const selectStepIssues =
  (stepId: string) =>
  (s: WorkflowState): ConsistencyIssue[] =>
    s.issuesByStep[stepId] ?? EMPTY_ISSUES

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
  selectStepRoster,
  selectRosterByStep,
  selectRoomStepMembers,
  selectRoomMembersByStep,
  selectNotesByStep,
  selectDocumentContentByDefId,
  selectLoading,
  selectError,
  selectDirty,
  selectDirtyStepIds,
  selectIssuesByStep,
  selectStepIssues,
  selectIsStale,
}
