import type { NormalizedMap } from '../lib'
import type { Workflow, WorkflowStep, WorkflowStepEdge, DocumentDef, RosterAgent, RoomStepMember } from '@/types/workflow'
import type { Document } from '@/types/document'

type WorkflowState = {
  items: NormalizedMap<Workflow>
  activeWorkflowId: string | null
  steps: NormalizedMap<WorkflowStep>
  edges: NormalizedMap<WorkflowStepEdge>
  documentsByStep: Record<string, Document[]>
  documentDefsByStep: Record<string, DocumentDef[]>
  rosterByStep: Record<string, RosterAgent[]>
  roomMembersByStep: Record<string, RoomStepMember[]>
  notesByStep: Record<string, string>
  documentContentByDefId: Record<string, string>
  dirtyStepIds: Set<string>
  loading: boolean
  error: string | null
  dirty: boolean
  lastFetched: number | null
}

export type { WorkflowState }
