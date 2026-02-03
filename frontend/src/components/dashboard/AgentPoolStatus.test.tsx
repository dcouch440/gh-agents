import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { AgentPoolStatus } from './AgentPoolStatus'
import type { Agent, AgentPoolStats } from '@/types'

const stats: AgentPoolStats = {
  orchestrators: { total: 2, available: 1, max: 3 },
  workers: { total: 4, available: 2, max: 5 },
  utilities: { total: 1, available: 1, max: 2 },
}

const agents: Agent[] = [
  { id: 'a1', name: 'Atlas', system_prompt: '', model_provider: 'anthropic', model_id: 'opus', model_max_tokens: 16384, model_temperature: 0.7, created_at: '', updated_at: '', tier: 'orchestrator', status: 'working' },
  { id: 'a2', name: 'Forge', system_prompt: '', model_provider: 'anthropic', model_id: 'sonnet', model_max_tokens: 8192, model_temperature: 0.7, created_at: '', updated_at: '', tier: 'worker', status: 'idle' },
  { id: 'a3', name: 'Shield', system_prompt: '', model_provider: 'anthropic', model_id: 'sonnet', model_max_tokens: 8192, model_temperature: 0.7, created_at: '', updated_at: '', tier: 'worker', status: 'waiting_for_context' },
]

describe('AgentPoolStatus', () => {
  it('renders tier labels', () => {
    render(<AgentPoolStatus agents={agents} stats={stats} />)
    expect(screen.getByText('ORCH')).toBeInTheDocument()
    expect(screen.getByText('WORK')).toBeInTheDocument()
    expect(screen.getByText('UTIL')).toBeInTheDocument()
  })

  it('shows capacity counts', () => {
    render(<AgentPoolStatus agents={agents} stats={stats} />)
    expect(screen.getByText('1/3')).toBeInTheDocument()
    expect(screen.getByText('2/5')).toBeInTheDocument()
    expect(screen.getByText('0/2')).toBeInTheDocument()
  })

  it('lists busy agents only', () => {
    render(<AgentPoolStatus agents={agents} stats={stats} />)
    expect(screen.getByText('Atlas')).toBeInTheDocument()
    expect(screen.getByText('Shield')).toBeInTheDocument()
    expect(screen.queryByText('Forge')).not.toBeInTheDocument()
  })
})
