type AgentStatus = 'active' | 'idle' | 'done'

type Agent = {
  id: string
  name: string
  status: AgentStatus
}

type ActivityLineStatus = 'running' | 'completed' | 'error'

type ActivityLine = {
  id: string
  toolName: string
  status: ActivityLineStatus
  summary: string
}

type AgentActivityModuleProps = {
  agents: Agent[]
  statusText: string | null
  activities: ActivityLine[]
  toolCallCount: number
}

const INDICATOR: Record<ActivityLineStatus, string> = {
  running: '\u27F3',
  completed: '\u2713',
  error: '\u2717',
}

function AgentActivityModule({ agents, statusText, activities, toolCallCount }: AgentActivityModuleProps) {
  if (agents.length === 0 && activities.length === 0) return null

  return (
    <>
    <div className="activity-module">
      {agents.length > 0 ? (
        <div className="activity-module__agents">
          {agents.map((agent) => (
            <span
              key={agent.id}
              className={`activity-module__agent activity-module__agent--${agent.status}`}
            >
              <span className="activity-module__agent-dot">{agent.status === 'done' ? '\u2713' : '\u25CF'}</span>{' '}
              {agent.name}
            </span>
          ))}
        </div>
      ) : null}

      {statusText !== null ? (
        <div className="activity-module__status">{statusText}</div>
      ) : null}

    </div>
    {activities.length > 0 ? (
      <div className="activity-module__tree">
        <div className="activity-module__feed">
          {activities.map((line, i) => {
            const len = activities.length
            let branch: string
            if (len === 1) {
              branch = '\u2500\u2500\u2500'
            } else if (i === 0) {
              branch = '\u250C\u2500\u2500'
            } else if (i === len - 1) {
              branch = '\u2514\u2500\u2500'
            } else {
              branch = '\u251C\u2500\u2500'
            }

            return (
              <div
                key={line.id}
                className={`activity-module__line activity-module__line--${line.status}`}
              >
                <span className="activity-module__line-branch">{branch}</span>
                <span className="activity-module__line-indicator">
                  {INDICATOR[line.status]}
                </span>
                <span className="activity-module__line-tool">{line.toolName}</span>
                <span className="activity-module__line-summary">{line.summary}</span>
              </div>
            )
          })}
        </div>
        <div className="activity-module__feed-footer">
          {toolCallCount} tool call{toolCallCount !== 1 ? 's' : ''}
        </div>
      </div>
    ) : null}
    </>
  )
}

export { AgentActivityModule }
export type { Agent, AgentStatus, ActivityLine, ActivityLineStatus, AgentActivityModuleProps }
