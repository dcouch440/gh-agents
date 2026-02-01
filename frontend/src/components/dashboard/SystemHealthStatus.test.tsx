import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { SystemHealthStatus } from './SystemHealthStatus'
import type { Config, AgentPoolStats } from '@/types'

const config: Config = {
  verbosity: 'normal',
  models: {
    orchestrator: { provider: 'anthropic', model_id: 'opus', max_tokens: 16384, temperature: 0.7 },
    worker: { provider: 'anthropic', model_id: 'sonnet', max_tokens: 8192, temperature: 0.7 },
    utility: { provider: 'anthropic', model_id: 'haiku', max_tokens: 4096, temperature: 0.3 },
  },
  pool: { max_orchestrators: 3, max_workers: 5, max_utilities: 2 },
  autonomy: 'full',
  git_strategy: 'branch',
  sandbox_mode: 'strict',
}

const stats: AgentPoolStats = {
  orchestrators: { total: 2, available: 1, max: 3 },
  workers: { total: 4, available: 2, max: 5 },
  utilities: { total: 1, available: 1, max: 2 },
}

describe('SystemHealthStatus', () => {
  it('renders config values', () => {
    render(<SystemHealthStatus config={config} agentStats={stats} wsConnected={true} />)
    expect(screen.getByText('full')).toBeInTheDocument()
    expect(screen.getByText('branch')).toBeInTheDocument()
    expect(screen.getByText('normal')).toBeInTheDocument()
    expect(screen.getByText('strict')).toBeInTheDocument()
  })

  it('shows connected when ws is up', () => {
    render(<SystemHealthStatus config={config} agentStats={stats} wsConnected={true} />)
    expect(screen.getByText('connected')).toBeInTheDocument()
  })

  it('shows disconnected when ws is down', () => {
    render(<SystemHealthStatus config={config} agentStats={stats} wsConnected={false} />)
    const el = screen.getByText('disconnected')
    expect(el).toBeInTheDocument()
    expect(el.classList.contains('sys-health__value--error')).toBe(true)
  })

  it('shows pool summary', () => {
    render(<SystemHealthStatus config={config} agentStats={stats} wsConnected={true} />)
    expect(screen.getByText(/1\/3 orch/)).toBeInTheDocument()
    expect(screen.getByText(/2\/5 work/)).toBeInTheDocument()
  })
})
