import type { NormalizedMap } from '../lib'
import type { Workflow, WorkflowStep, WorkflowStepEdge, DocumentDef, RosterAgent } from '@/types/workflow'
import type { Document } from '@/types/document'

type WorkflowState = {
  items: NormalizedMap<Workflow>
  activeWorkflowId: string | null
  steps: NormalizedMap<WorkflowStep>
  edges: NormalizedMap<WorkflowStepEdge>
  documentsByStep: Record<string, Document[]>
  documentDefsByStep: Record<string, DocumentDef[]>
  rosterByStep: Record<string, RosterAgent[]>
  dirtyStepIds: Set<string>
  loading: boolean
  error: string | null
  dirty: boolean
  lastFetched: number | null
}

export type { WorkflowState }
