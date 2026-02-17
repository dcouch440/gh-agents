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
): DynamicNodeExecutionState => {
  const stepExec = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepState(nodeId))
  const stepExecStatus = toExecutionStatus(stepExec?.status)

  const agentSourceStatus = useStore(
    stepStreamStore.store,
    isAgent
      ? (s) => s.sources[rosterAgentId ?? '']?.status ?? 'idle'
      : () => 'idle' as const,
  )

  const isExecuting = isAgent ? agentSourceStatus !== 'idle' : stepExecStatus !== 'idle'
  const resolvedExecStatus = isAgent
    ? toExecutionStatus(agentSourceStatus)
    : stepExecStatus

  return { isExecuting, resolvedExecStatus, agentSourceStatus, stepExecStatus }
}

export { useDynamicNodeExecution }
export type { DynamicNodeExecutionState }
