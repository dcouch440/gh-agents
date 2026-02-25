import type { NormalizedMap } from '../lib'
import type { Workflow, WorkflowStep, WorkflowStepEdge, RosterAgent, RoomStepMember, StepQuestionState } from '@/types/workflow'
import type { Document } from '@/types/document'

type WorkflowState = {
  items: NormalizedMap<Workflow>
  activeWorkflowId: string | null
  steps: NormalizedMap<WorkflowStep>
  edges: NormalizedMap<WorkflowStepEdge>
  documentsByStep: Record<string, Document[]>
  rosterByStep: Record<string, RosterAgent[]>
  roomMembersByStep: Record<string, RoomStepMember[]>
  planByStep: Record<string, string>
  questionStateByStep: Record<string, StepQuestionState>
  dirtyStepIds: Set<string>
  loading: boolean
  error: string | null
  dirty: boolean
  lastFetched: number | null
}

export type { WorkflowState }
