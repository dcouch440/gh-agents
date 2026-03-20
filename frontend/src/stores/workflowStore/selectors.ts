import { toArray, nmGet } from '../lib'
import type { WorkflowState } from './types'
import type { Workflow, WorkflowStep, WorkflowStepEdge, RosterAgent, RoomStepMember, StepQuestionState } from '@/types/workflow'
import type { Document } from '@/types/document'
import { STALE_THRESHOLD_MS } from './_store'

const EMPTY_DOCS: Document[] = []
const EMPTY_ROSTER: RosterAgent[] = []
const EMPTY_ROOM_MEMBERS: RoomStepMember[] = []

import { memoFactory } from '../lib'

const selectAll = (s: WorkflowState): Workflow[] => toArray(s.items)

const selectById = memoFactory(
  (id: string) =>
  (s: WorkflowState): Workflow | undefined =>
    nmGet(s.items, id),
)

const selectActiveWorkflowId = (s: WorkflowState): string | null => s.activeWorkflowId

const selectSteps = (s: WorkflowState): WorkflowStep[] => toArray(s.steps)

const selectEdges = (s: WorkflowState): WorkflowStepEdge[] => toArray(s.edges)

const selectStepById = memoFactory(
  (id: string | null) =>
  (s: WorkflowState): WorkflowStep | null =>
    id !== null ? (nmGet(s.steps, id) ?? null) : null,
)

const selectEdgeById = memoFactory(
  (id: string | null) =>
  (s: WorkflowState): WorkflowStepEdge | null =>
    id !== null ? (nmGet(s.edges, id) ?? null) : null,
)

const selectStepDocuments = memoFactory(
  (stepId: string) =>
  (s: WorkflowState): Document[] =>
    s.documentsByStep[stepId] ?? EMPTY_DOCS,
)

const selectStepRoster = memoFactory(
  (stepId: string) =>
  (s: WorkflowState): RosterAgent[] =>
    s.rosterByStep[stepId] ?? EMPTY_ROSTER,
)

const selectRosterByStep = (s: WorkflowState): Record<string, RosterAgent[]> =>
  s.rosterByStep

const selectRoomStepMembers = memoFactory(
  (stepId: string) =>
  (s: WorkflowState): RoomStepMember[] =>
    s.roomMembersByStep[stepId] ?? EMPTY_ROOM_MEMBERS,
)

const selectRoomMembersByStep = (s: WorkflowState): Record<string, RoomStepMember[]> =>
  s.roomMembersByStep

const selectPlanByStep = (s: WorkflowState): Record<string, string> =>
  s.planByStep

const selectLoading = (s: WorkflowState): boolean => s.loading

const selectError = (s: WorkflowState): string | null => s.error

const selectDirty = (s: WorkflowState): boolean => s.dirty

const selectDirtyStepIds = (s: WorkflowState): Set<string> => s.dirtyStepIds

const selectRosterAgentById = memoFactory(
  (id: string) =>
  (s: WorkflowState): RosterAgent | null => {
    for (const roster of Object.values(s.rosterByStep)) {
      const found = roster.find((a) => a.id === id)
      if (found) return found
    }
    return null
  },
)

const selectRoomMemberById = memoFactory(
  (id: string) =>
  (s: WorkflowState): RoomStepMember | null => {
    for (const members of Object.values(s.roomMembersByStep)) {
      const found = members.find((m) => m.id === id)
      if (found) return found
    }
    return null
  },
)

const selectStepQuestionState = memoFactory(
  (stepId: string) =>
  (s: WorkflowState): StepQuestionState | null =>
    s.questionStateByStep[stepId] ?? null,
)

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
  selectStepRoster,
  selectRosterByStep,
  selectRoomStepMembers,
  selectRoomMembersByStep,
  selectPlanByStep,
  selectLoading,
  selectError,
  selectDirty,
  selectDirtyStepIds,
  selectRosterAgentById,
  selectRoomMemberById,
  selectStepQuestionState,
  selectIsStale,
}
