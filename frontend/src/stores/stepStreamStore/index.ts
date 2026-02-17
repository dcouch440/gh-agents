import { store } from './_store'
import { handleWsEvent } from './wsHandler'
import type { StepStreamState, SourceStreamState } from './types'

const selectSource = (sourceId: string) => (s: StepStreamState): SourceStreamState | null => s.sources[sourceId] ?? null

const selectAllSources = (s: StepStreamState): Record<string, SourceStreamState> => s.sources

const selectSourcesForStep = (stepId: string) => (s: StepStreamState): SourceStreamState[] =>
  Object.values(s.sources).filter((src) => src.stepId === stepId)

const selectDesignerStatus = (s: StepStreamState): StepStreamState['designerStatus'] => s.designerStatus

const selectActiveStepId = (s: StepStreamState): string | null => s.activeStepId

export const stepStreamStore = {
  store,
  handleWsEvent,
  selectSource,
  selectAllSources,
  selectSourcesForStep,
  selectDesignerStatus,
  selectActiveStepId,
}

export type { StepStreamState, SourceStreamState, SourceStreamStatus, StreamToolUse } from './types'
