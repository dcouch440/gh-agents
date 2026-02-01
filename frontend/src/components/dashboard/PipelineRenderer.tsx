import { PipelineStageNode } from './PipelineStageNode'
import type { StageStatus } from './PipelineStageNode'
import type { Pipeline, PipelineRun, StageExecution } from '@/types'

type PipelineRendererProps = {
  pipeline: Pipeline
  run: PipelineRun | null
  stages: StageExecution[]
}

const RUN_STATUS_LABEL: Record<string, string> = {
  running: 'RUNNING',
  waiting_for_approval: 'AWAITING APPROVAL',
  completed: 'DONE',
  failed: 'FAILED',
}

const deriveStageStatus = (
  stageNumber: number,
  currentStage: number,
  execution: StageExecution | null,
  runStatus: string,
): StageStatus => {
  if (execution === null) {
    return stageNumber <= currentStage ? 'pending' : 'pending'
  }
  if (execution.status === 'completed') return 'completed'
  if (execution.status === 'failed') return 'failed'
  if (stageNumber === currentStage && runStatus === 'waiting_for_approval') return 'waiting_for_approval'
  if (stageNumber === currentStage) return 'running'
  return 'pending'
}

const formatTokens = (n: number): string => {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return `${n}`
}

const formatElapsed = (startedAt: string, completedAt: string | null): string => {
  const start = new Date(startedAt).getTime()
  const end = completedAt !== null ? new Date(completedAt).getTime() : Date.now()
  const sec = (end - start) / 1000
  if (sec < 60) return `${sec.toFixed(0)}s`
  return `${(sec / 60).toFixed(1)}m`
}

function PipelineRenderer({ pipeline, run, stages }: PipelineRendererProps) {
  if (run === null) {
    return <div className="pipeline pipeline--empty">no active run</div>
  }

  const executionMap = new Map<number, StageExecution>()
  for (const exec of stages) {
    executionMap.set(exec.stage_number, exec)
  }

  return (
    <div className="pipeline">
      <div className="pipeline__header">
        PIPELINE: {pipeline.name}{'  '}
        RUN: {RUN_STATUS_LABEL[run.status] ?? run.status}{'  '}
        STAGE: {run.current_stage}/{pipeline.stages.length}
      </div>

      <div className="pipeline__flow">
        {pipeline.stages.map((stage, i) => {
          const exec = executionMap.get(stage.stage_number) ?? null
          const status = deriveStageStatus(stage.stage_number, run.current_stage, exec, run.status)

          return (
            <div key={stage.stage_number} className="pipeline__stage-group">
              <PipelineStageNode
                stageNumber={stage.stage_number}
                stageName={stage.stage_name}
                status={status}
                approvalRequired={stage.approval_required}
                agentName={exec?.agent_id ?? null}
                durationMs={exec?.duration_ms ?? null}
                tokenCount={exec !== null ? exec.input_tokens + exec.output_tokens : null}
                isCurrent={stage.stage_number === run.current_stage}
              />
              {i < pipeline.stages.length - 1 ? (
                <span className={`pipeline__connector ${stage.stage_number < run.current_stage ? 'pipeline__connector--done' : ''}`}>
                  {pipeline.stages[i + 1]?.approval_required ? '\u2500\u2500\u2298\u2500\u2500\u25B8' : '\u2500\u2500\u2500\u25B8'}
                </span>
              ) : null}
            </div>
          )
        })}
      </div>

      <div className="pipeline__footer">
        TOKENS: {formatTokens(run.total_input_tokens)}in / {formatTokens(run.total_output_tokens)}out{'  '}
        ELAPSED: {formatElapsed(run.started_at, run.completed_at)}
      </div>
    </div>
  )
}

export { PipelineRenderer }
export type { PipelineRendererProps }
