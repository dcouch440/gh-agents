import { WORKFLOW_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import type {
  WorkforceDesignerProgressData,
  WorkforceAgentProgressData,
  DesignerAgentDesignedData,
  StepStreamTokenData,
  StepStreamToolStartData,
  StepStreamToolEndData,
  StepStreamErrorData,
} from '@/types/ws'
import type { SourceStreamState, SourceStreamStatus } from './types'
import { store, initialState, makeDefaultSourceState, updateSource } from './_store'

const agentStatusToSourceStatus = (status: string): SourceStreamStatus => {
  switch (status) {
    case 'started': return 'running'
    case 'completed': return 'completed'
    case 'failed': return 'failed'
    default: return 'idle'
  }
}

const handleWsEvent = (msg: WsWireMessage): void => {
  try {
    switch (msg.event) {
      case WORKFLOW_EVENT.STARTED: {
        store.setState({ ...initialState })
        break
      }
      case WORKFLOW_EVENT.WORKFORCE_DESIGNER_PROGRESS: {
        const d = msg.data as WorkforceDesignerProgressData
        const designerStatus =
          d.status === 'started' ? 'running' as const
            : d.status === 'completed' ? 'completed' as const
              : d.status === 'failed' ? 'failed' as const
                : 'idle' as const
        store.setState((s) => {
          const existing = s.designStatusByStep[d.step_id]
          const stepStatus = designerStatus === 'running'
            ? { status: 'running' as const, designedCount: 0, totalCount: existing?.totalCount ?? 0, lastAgentName: null }
            : { status: designerStatus, designedCount: existing?.designedCount ?? 0, totalCount: existing?.totalCount ?? 0, lastAgentName: existing?.lastAgentName ?? null }
          return {
            designerStatus,
            activeStepId: d.step_id,
            designStatusByStep: { ...s.designStatusByStep, [d.step_id]: stepStatus },
          }
        })
        break
      }
      case WORKFLOW_EVENT.DESIGNER_AGENT_DESIGNED: {
        const d = msg.data as DesignerAgentDesignedData
        store.setState((s) => ({
          designStatusByStep: {
            ...s.designStatusByStep,
            [d.step_id]: {
              status: 'running' as const,
              designedCount: d.designed_count,
              totalCount: d.total_count,
              lastAgentName: d.agent_name,
            },
          },
        }))
        break
      }
      case WORKFLOW_EVENT.WORKFORCE_AGENT_PROGRESS: {
        const d = msg.data as WorkforceAgentProgressData
        const status = agentStatusToSourceStatus(d.status)
        store.setState((s) => {
          const existing = s.sources[d.roster_agent_id]
          if (existing) {
            return {
              sources: updateSource(s.sources, d.roster_agent_id, {
                status,
                startedAt: status === 'running' ? msg.ts : existing.startedAt,
                completedAt: status === 'completed' || status === 'failed' ? msg.ts : existing.completedAt,
              }),
              activeStepId: d.step_id,
            }
          }
          const fresh: SourceStreamState = {
            ...makeDefaultSourceState(d.roster_agent_id, d.agent_name, d.step_id),
            status,
            startedAt: status === 'running' ? msg.ts : null,
            completedAt: status === 'completed' || status === 'failed' ? msg.ts : null,
          }
          return {
            sources: { ...s.sources, [d.roster_agent_id]: fresh },
            activeStepId: d.step_id,
          }
        })
        break
      }
      case WORKFLOW_EVENT.STEP_STREAM_TOKEN: {
        const d = msg.data as StepStreamTokenData
        store.setState((s) => {
          const existing = s.sources[d.source_id]
          if (!existing) {
            const fresh = makeDefaultSourceState(d.source_id, d.source_name, d.step_id)
            return { sources: { ...s.sources, [d.source_id]: { ...fresh, status: 'running', streamBuffer: d.content } } }
          }
          return { sources: updateSource(s.sources, d.source_id, { streamBuffer: existing.streamBuffer + d.content }) }
        })
        break
      }
      case WORKFLOW_EVENT.STEP_STREAM_TOOL_START: {
        const d = msg.data as StepStreamToolStartData
        store.setState((s) => {
          const existing = s.sources[d.source_id]
          if (!existing) return {}
          return {
            sources: updateSource(s.sources, d.source_id, {
              toolUses: [...existing.toolUses, { toolName: d.tool_name, toolId: d.tool_id, status: 'running', startedAt: msg.ts }],
            }),
          }
        })
        break
      }
      case WORKFLOW_EVENT.STEP_STREAM_TOOL_END: {
        const d = msg.data as StepStreamToolEndData
        store.setState((s) => {
          const existing = s.sources[d.source_id]
          if (!existing) return {}
          return {
            sources: updateSource(s.sources, d.source_id, {
              toolUses: existing.toolUses.map((t) =>
                t.toolId === d.tool_id ? { ...t, status: 'completed' as const } : t,
              ),
            }),
          }
        })
        break
      }
      case WORKFLOW_EVENT.STEP_STREAM_ERROR: {
        const d = msg.data as StepStreamErrorData
        store.setState((s) => ({
          sources: updateSource(s.sources, d.source_id, { error: d.error }),
        }))
        break
      }
    }
  } catch (err) {
    console.error(`[stepStreamStore] WS handler error on "${msg.event}":`, err)
  }
}

export { handleWsEvent }
