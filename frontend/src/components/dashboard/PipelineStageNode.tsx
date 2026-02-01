type StageStatus = 'pending' | 'running' | 'completed' | 'failed' | 'waiting_for_approval' | 'skipped'

type PipelineStageNodeProps = {
  stageNumber: number
  stageName: string
  status: StageStatus
  approvalRequired: boolean
  agentName: string | null
  durationMs: number | null
  tokenCount: number | null
  isCurrent: boolean
}

const STATUS_INDICATOR: Record<StageStatus, string> = {
  pending: '\u00B7',
  running: '\u27F3',
  completed: '\u2713',
  failed: '\u2717',
  waiting_for_approval: '\u25C6',
  skipped: '\u2013',
}

const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

const formatTokens = (n: number): string => {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return `${n}`
}

function PipelineStageNode({
  stageNumber,
  stageName,
  status,
  approvalRequired,
  agentName,
  durationMs,
  tokenCount,
  isCurrent,
}: PipelineStageNodeProps) {
  const mod = isCurrent ? 'stage-node--current' : `stage-node--${status}`

  return (
    <div className={`stage-node ${mod}`}>
      <div className="stage-node__header">
        <span className="stage-node__border">[</span>
        <span className="stage-node__label">
          {approvalRequired ? <span className="stage-node__gate">{'\u2298'} </span> : null}
          {stageNumber}: {stageName}
        </span>
        <span className={`stage-node__indicator stage-node__indicator--${status}`}>
          {STATUS_INDICATOR[status]}
        </span>
        <span className="stage-node__border">]</span>
      </div>
      {(agentName !== null || durationMs !== null || tokenCount !== null) ? (
        <div className="stage-node__meta">
          {agentName !== null ? <span className="stage-node__agent">{agentName}</span> : null}
          {durationMs !== null ? <span className="stage-node__duration">{formatDuration(durationMs)}</span> : null}
          {tokenCount !== null ? <span className="stage-node__tokens">{formatTokens(tokenCount)} tok</span> : null}
        </div>
      ) : null}
    </div>
  )
}

export { PipelineStageNode }
export type { StageStatus, PipelineStageNodeProps }
