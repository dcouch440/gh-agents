import type { Config, AgentPoolStats } from '@/types'

type SystemHealthStatusProps = {
  config: Config
  agentStats: AgentPoolStats
  wsConnected: boolean
}

function SystemHealthStatus({ config, agentStats, wsConnected }: SystemHealthStatusProps) {
  // AgentPoolStats structure is defined in types/agent.ts
  const poolStr = `${agentStats.total - agentStats.available}/${agentStats.max} agents`

  return (
    <div className="sys-health">
      <div className="sys-health__row">
        <span className="sys-health__label">MODE</span>
        <span className="sys-health__value">{config.autonomy}</span>
      </div>
      <div className="sys-health__row">
        <span className="sys-health__label">GIT</span>
        <span className="sys-health__value">{config.git_strategy}</span>
      </div>
      <div className="sys-health__row">
        <span className="sys-health__label">VERBOSITY</span>
        <span className="sys-health__value">{config.verbosity}</span>
      </div>
      <div className="sys-health__row">
        <span className="sys-health__label">SANDBOX</span>
        <span className="sys-health__value">{config.sandbox_mode}</span>
      </div>
      <div className="sys-health__row">
        <span className="sys-health__label">WS</span>
        <span className={`sys-health__value ${wsConnected ? 'sys-health__value--ok' : 'sys-health__value--error'}`}>
          {wsConnected ? 'connected' : 'disconnected'}
        </span>
      </div>
      <div className="sys-health__row">
        <span className="sys-health__label">POOL</span>
        <span className="sys-health__value">{poolStr}</span>
      </div>
    </div>
  )
}

export { SystemHealthStatus }
export type { SystemHealthStatusProps }
