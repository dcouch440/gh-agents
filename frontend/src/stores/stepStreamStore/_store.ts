import { createStore } from '../lib'
import type { StepStreamState, SourceStreamState } from './types'

const initialState: StepStreamState = {
  sources: {},
  designerStatus: 'idle',
  activeStepId: null,
}

const store = createStore<StepStreamState>(() => ({ ...initialState }))

const makeDefaultSourceState = (sourceId: string, sourceName: string, stepId: string): SourceStreamState => ({
  sourceId,
  sourceName,
  stepId,
  status: 'idle',
  streamBuffer: '',
  toolUses: [],
  error: null,
  startedAt: null,
  completedAt: null,
})

const updateSource = (
  sources: Record<string, SourceStreamState>,
  sourceId: string,
  patch: Partial<SourceStreamState>,
): Record<string, SourceStreamState> => {
  const existing = sources[sourceId]
  if (!existing) return sources
  return { ...sources, [sourceId]: { ...existing, ...patch } }
}

export { store, initialState, makeDefaultSourceState, updateSource }
