import { createStore, createNormalizedMap, logger } from '../lib'
import type { WorkflowState } from './types'
import type { Workflow, WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

const STALE_THRESHOLD_MS = 60_000

const store = logger(
  'workflowStore',
  createStore<WorkflowState>(() => ({
    items: createNormalizedMap<Workflow>(),
    activeWorkflowId: null,
    steps: createNormalizedMap<WorkflowStep>(),
    edges: createNormalizedMap<WorkflowStepEdge>(),
    documentsByStep: {},
    documentDefsByStep: {},
    rosterByStep: {},
    dirtyStepIds: new Set<string>(),
    loading: false,
    error: null,
    dirty: false,
    lastFetched: null,
  })),
)

const getActiveId = (): string | null => store.getState().activeWorkflowId

export { store, getActiveId, STALE_THRESHOLD_MS }
