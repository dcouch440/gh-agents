import type { Agent, AgentPoolStats, AgentTier } from '@/types'

type AgentPoolStatusProps = {
  agents: Agent[]
  stats: AgentPoolStats
}

type TierKey = 'orchestrators' | 'workers' | 'utilities'

const TIER_MAP: { tier: AgentTier; key: TierKey; label: string }[] = [
  { tier: 'orchestrator', key: 'orchestrators', label: 'ORCH' },
  { tier: 'worker', key: 'workers', label: 'WORK' },
  { tier: 'utility', key: 'utilities', label: 'UTIL' },
]

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

  return (
    <div className="pool-status">
      {TIER_MAP.map(({ key, label }) => {
        const s = stats[key]
        const active = s.total - s.available
        return (
          <div key={key} className="pool-status__tier">
            <span className="pool-status__label">{label}</span>
            <span className="pool-status__bar">
              <span className="pool-status__bar-fill">{buildBar(active, s.max)}</span>
            </span>
            <span className="pool-status__count">{active}/{s.max}</span>
          </div>
        )
      })}

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
