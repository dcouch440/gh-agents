import type { Pipeline, PipelineRun, StageExecution } from '@/types'
import type { TreeData, TreeNode, TreeEdgeData, NodeStatus } from '../types'

type PipelineMeta = {
  stageNumber: number
  approvalRequired: boolean
  agentId: string | null
  durationMs: number | null
  inputTokens: number
  outputTokens: number
}

const deriveStatus = (
  stageNumber: number,
  currentStage: number,
  execution: StageExecution | null,
  runStatus: string,
): NodeStatus => {
  if (execution === null) return 'pending'
  if (execution.status === 'completed') return 'completed'
  if (execution.status === 'failed') return 'failed'
  if (stageNumber === currentStage && runStatus === 'waiting_for_approval') return 'waiting'
  if (stageNumber === currentStage) return 'running'
  return 'pending'
}

const pipelineToTree = (
  pipeline: Pipeline,
  run: PipelineRun | null,
  executions: StageExecution[],
): TreeData<PipelineMeta> => {
  const execMap = new Map<number, StageExecution>()
  for (const exec of executions) {
    execMap.set(exec.stage_number, exec)
  }

  const nodes: Record<string, TreeNode<PipelineMeta>> = {}
  const edges: TreeEdgeData[] = []
  const rootIds: string[] = []

  for (let i = 0; i < pipeline.stages.length; i++) {
    const stage = pipeline.stages[i]!
    const id = `stage-${stage.stage_number}`
    const exec = execMap.get(stage.stage_number) ?? null

    const status = run !== null
      ? deriveStatus(stage.stage_number, run.current_stage, exec, run.status)
      : 'pending'

    const nextId = i < pipeline.stages.length - 1
      ? `stage-${pipeline.stages[i + 1]!.stage_number}`
      : null

    nodes[id] = {
      id,
      label: stage.stage_name,
      status,
      children: nextId !== null ? [nextId] : [],
      metadata: {
        stageNumber: stage.stage_number,
        approvalRequired: stage.approval_required,
        agentId: exec?.agent_id ?? null,
        durationMs: exec?.duration_ms ?? null,
        inputTokens: exec?.input_tokens ?? 0,
        outputTokens: exec?.output_tokens ?? 0,
      },
    }

    if (i === 0) rootIds.push(id)

    if (nextId !== null) {
      const nextStage = pipeline.stages[i + 1]!
      edges.push({
        sourceId: id,
        targetId: nextId,
        label: null,
        variant: nextStage.approval_required ? 'approval' : 'normal',
      })
    }
  }

  return { nodes, rootIds, edges }
}

export { pipelineToTree }
export type { PipelineMeta }
