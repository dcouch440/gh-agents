import { store } from './_store'
import { handleWsEvent } from './wsHandler'
import type { StepStreamState, SourceStreamState, StepDesignState } from './types'

const selectSource = (sourceId: string) => (s: StepStreamState): SourceStreamState | null => s.sources[sourceId] ?? null

const selectAllSources = (s: StepStreamState): Record<string, SourceStreamState> => s.sources

const selectSourcesForStep = (stepId: string) => (s: StepStreamState): SourceStreamState[] =>
  Object.values(s.sources).filter((src) => src.stepId === stepId)

const selectDesignerStatus = (s: StepStreamState): StepStreamState['designerStatus'] => s.designerStatus

const selectActiveStepId = (s: StepStreamState): string | null => s.activeStepId

const selectDesignStatusForStep = (stepId: string) => (s: StepStreamState): StepDesignState | null =>
  s.designStatusByStep[stepId] ?? null

const selectDesignStatusByStep = (s: StepStreamState): Record<string, StepDesignState> => s.designStatusByStep

const selectIsAgentDesigned = (stepId: string, agentSlug: string) => (s: StepStreamState): boolean =>
  s.designStatusByStep[stepId]?.designedAgentSlugs.has(agentSlug) === true

export const stepStreamStore = {
  store,
  handleWsEvent,
  selectSource,
  selectAllSources,
  selectSourcesForStep,
  selectDesignerStatus,
  selectActiveStepId,
  selectDesignStatusForStep,
  selectDesignStatusByStep,
  selectIsAgentDesigned,
}

export type { StepStreamState, SourceStreamState, SourceStreamStatus, StreamToolUse, StepDesignState } from './types'
