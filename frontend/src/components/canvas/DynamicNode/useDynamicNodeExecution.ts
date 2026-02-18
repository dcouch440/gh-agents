import { useStore, workflowExecutionStore, stepStreamStore } from '@/stores'
import { toExecutionStatus } from '../execution'
import type { ExecutionStatus } from '../execution'

type DynamicNodeExecutionState = {
  isExecuting: boolean
  resolvedExecStatus: ExecutionStatus
  agentSourceStatus: string
  stepExecStatus: ExecutionStatus
}

const useDynamicNodeExecution = (
  nodeId: string,
  isAgent: boolean,
  rosterAgentId: string | null,
  protocolStepId?: string | null,
): DynamicNodeExecutionState => {
  const stepExec = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepState(nodeId))
  const stepExecStatus = toExecutionStatus(stepExec?.status)

  // For agent nodes, also check the parent protocol step's completion state.
  // After page refresh, the ephemeral stream store is empty, but the parent
  // step's persisted state tells us the agent already ran.
  const parentStepExec = useStore(
    workflowExecutionStore.store,
    isAgent && protocolStepId ? workflowExecutionStore.selectStepState(protocolStepId) : () => undefined,
  )
  const parentCompleted = parentStepExec?.status === 'success'

  const agentSourceStatus = useStore(
    stepStreamStore.store,
    isAgent
      ? (s) => s.sources[rosterAgentId ?? '']?.status ?? 'idle'
      : () => 'idle' as const,
  )

  const isExecuting = isAgent ? agentSourceStatus !== 'idle' || parentCompleted : stepExecStatus !== 'idle'
  const resolvedExecStatus = isAgent
    ? (agentSourceStatus !== 'idle' ? toExecutionStatus(agentSourceStatus) : (parentCompleted ? 'completed' : 'idle'))
    : stepExecStatus

  return { isExecuting, resolvedExecStatus, agentSourceStatus, stepExecStatus }
}

export { useDynamicNodeExecution }
export type { DynamicNodeExecutionState }
