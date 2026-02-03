import type { Agent, AgentPoolStats } from '@/types'

type AgentPoolStatusProps = {
  agents: Agent[]
  stats: AgentPoolStats
}

const STATUS_DOT: Record<string, string> = {
  idle: '\u25CB',
  working: '\u25CF',
  waiting_for_context: '\u27F3',
  waiting_for_approval: '\u25C6',
}

const buildBar = (active: number, max: number): string => {
  const filled = '#'.repeat(active)
  const empty = '-'.repeat(max - active)
  return `[${filled}${empty}]`
}

function AgentPoolStatus({ agents, stats }: AgentPoolStatusProps) {
  const busy = agents.filter((a) => a.status && a.status !== 'idle')
  const active = stats.total - stats.available

  return (
    <div className="pool-status">
      <div className="pool-status__tier">
        <span className="pool-status__label">AGENTS</span>
        <span className="pool-status__bar">
          <span className="pool-status__bar-fill">{buildBar(active, stats.max)}</span>
        </span>
        <span className="pool-status__count">{active}/{stats.max}</span>
      </div>

      {busy.length > 0 ? (
        <div className="pool-status__agents">
          {busy.map((a) => (
            <div key={a.id} className={`pool-status__agent pool-status__agent--${(a.status ?? 'idle').replace('waiting_for_', 'waiting-')}`}>
              <span className="pool-status__dot">{STATUS_DOT[a.status ?? 'idle'] ?? '\u25CB'}</span>{' '}
              {a.name}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  )
}

export { AgentPoolStatus }
export type { AgentPoolStatusProps }
