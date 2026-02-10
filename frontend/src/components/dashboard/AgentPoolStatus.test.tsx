import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { AgentPoolStatus } from './AgentPoolStatus'
import type { Agent, AgentPoolStats } from '@/types'

const stats: AgentPoolStats = {
  total: 7,
  available: 4,
  max: 10,
}

const agents: Agent[] = [
  {
    id: 'a1',
    name: 'Atlas',
    system_prompt: '',
    model_provider: 'anthropic',
    model_id: 'opus',
    model_max_tokens: 16384,
    model_temperature: 0.7,
    status: 'working',
    version: 1,
  },
  {
    id: 'a2',
    name: 'Forge',
    system_prompt: '',
    model_provider: 'anthropic',
    model_id: 'sonnet',
    model_max_tokens: 8192,
    model_temperature: 0.7,
    status: 'idle',
    version: 1,
  },
  {
    id: 'a3',
    name: 'Shield',
    system_prompt: '',
    model_provider: 'anthropic',
    model_id: 'sonnet',
    model_max_tokens: 8192,
    model_temperature: 0.7,
    status: 'waiting_for_context',
    version: 1,
  },
]

describe('AgentPoolStatus', () => {
  it('renders agent pool label', () => {
    render(<AgentPoolStatus agents={agents} stats={stats} />)
    expect(screen.getByText('AGENTS')).toBeInTheDocument()
  })

  it('shows capacity count', () => {
    render(<AgentPoolStatus agents={agents} stats={stats} />)
    expect(screen.getByText('3/10')).toBeInTheDocument()
  })

  it('lists busy agents only', () => {
    render(<AgentPoolStatus agents={agents} stats={stats} />)
    expect(screen.getByText('Atlas')).toBeInTheDocument()
    expect(screen.getByText('Shield')).toBeInTheDocument()
    expect(screen.queryByText('Forge')).not.toBeInTheDocument()
  })
})
