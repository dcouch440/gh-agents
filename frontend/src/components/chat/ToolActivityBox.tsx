type ToolStatus = 'pending' | 'running' | 'completed' | 'error'

type ToolActivityBoxProps = {
  toolName: string
  status: ToolStatus
  durationMs: number | null
  detail?: string
}

const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

const CIRCLE_CONTENT: Record<ToolStatus, string> = {
  pending: '',
  running: '',
  completed: '\u2713',
  error: '\u2717',
}

function ToolActivityBox({ toolName, status, durationMs, detail }: ToolActivityBoxProps) {
  return (
    <div className={`tool-tile tool-tile--${status}`}>
      <div className="tool-tile__row">
        <span className="tool-tile__circle">{CIRCLE_CONTENT[status]}</span>
        <span className="tool-tile__name">{toolName}</span>
        {durationMs !== null ? (
          <span className="tool-tile__duration">{formatDuration(durationMs)}</span>
        ) : null}
      </div>
      {detail && status !== 'completed' ? (
        <div className="tool-tile__detail">{detail}</div>
      ) : null}
    </div>
  )
}

export { ToolActivityBox }
export type { ToolStatus, ToolActivityBoxProps }
