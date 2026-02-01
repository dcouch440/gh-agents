type ToolStatus = 'pending' | 'running' | 'completed' | 'error'

type ToolActivityBoxProps = {
  toolName: string
  status: ToolStatus
  durationMs: number | null
  detail?: string
  progress?: number
}

const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

const PREFIX: Record<ToolStatus, string> = {
  pending: '~',
  running: '>',
  completed: '\u2713',
  error: '\u2717',
}

function ToolActivityBox({ toolName, status, durationMs, detail, progress }: ToolActivityBoxProps) {
  return (
    <div className={`tool-box tool-box--${status}`}>
      <div className="tool-box__header">
        <span className="tool-box__name">
          <span className="tool-box__prefix">{PREFIX[status]}</span>
          {toolName}
        </span>
        {durationMs !== null ? (
          <span className="tool-box__duration">{formatDuration(durationMs)}</span>
        ) : null}
      </div>
      {status !== 'completed' ? (
        <div className="tool-box__progress">
          <div
            className="tool-box__progress-bar"
            style={{ width: `${progress ?? 0}%` }}
          />
        </div>
      ) : null}
      {detail && status !== 'completed' ? (
        <div className="tool-box__detail">{detail}</div>
      ) : null}
    </div>
  )
}

export { ToolActivityBox }
export type { ToolStatus, ToolActivityBoxProps }
