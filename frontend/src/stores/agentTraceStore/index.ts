import { store, reset } from './_store'
import { handleWsEvent } from './wsHandler'
import { hydrateFromTimeline, setHydratedRun } from './hydrate'
import type { AgentTraceState, AgentTrace } from './types'

const selectTraces = (s: AgentTraceState): Record<string, AgentTrace> => s.traces

const selectOrder = (s: AgentTraceState): string[] => s.order

const selectHydratedRunId = (s: AgentTraceState): string | null => s.hydratedRunId

/** Run whose timeline is already fetched in full. See `AgentTraceState`. */
const selectTimelineRunId = (s: AgentTraceState): string | null => s.timelineRunId

const selectTraceById = (id: string) => (s: AgentTraceState): AgentTrace | null => s.traces[id] ?? null

export const agentTraceStore = {
  store,
  handleWsEvent,
  hydrateFromTimeline,
  setHydratedRun,
  reset,
  selectTraces,
  selectOrder,
  selectHydratedRunId,
  selectTimelineRunId,
  selectTraceById,
}

export type { AgentTraceState, AgentTrace, AgentTraceEvent } from './types'
