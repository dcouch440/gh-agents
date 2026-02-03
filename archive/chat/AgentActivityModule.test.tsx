import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { AgentActivityModule } from './AgentActivityModule'
import type { Agent, ActivityLine } from './AgentActivityModule'

const agents: Agent[] = [
  { id: 'a1', name: 'Atlas', status: 'active' },
  { id: 'a2', name: 'Forge', status: 'idle' },
  { id: 'a3', name: 'Shield', status: 'done' },
]

const activities: ActivityLine[] = [
  { id: 't1', toolName: 'search_files', status: 'completed', summary: 'found 3 matches' },
  { id: 't2', toolName: 'read_file', status: 'running', summary: 'reading src/auth/mod.rs' },
]

describe('AgentActivityModule', () => {
  it('renders agent pills', () => {
    render(
      <AgentActivityModule agents={agents} statusText={null} activities={[]} toolCallCount={0} />,
    )
    expect(screen.getByText('Atlas')).toBeInTheDocument()
    expect(screen.getByText('Forge')).toBeInTheDocument()
    expect(screen.getByText('Shield')).toBeInTheDocument()
  })

  it('applies agent status classes', () => {
    const { container } = render(
      <AgentActivityModule agents={agents} statusText={null} activities={[]} toolCallCount={0} />,
    )
    expect(container.querySelector('.activity-module__agent--active')).toBeInTheDocument()
    expect(container.querySelector('.activity-module__agent--idle')).toBeInTheDocument()
    expect(container.querySelector('.activity-module__agent--done')).toBeInTheDocument()
  })

  it('renders status text', () => {
    render(
      <AgentActivityModule
        agents={agents}
        statusText="Forge is writing middleware"
        activities={[]}
        toolCallCount={0}
      />,
    )
    expect(screen.getByText('Forge is writing middleware')).toBeInTheDocument()
  })

  it('hides status text when null', () => {
    const { container } = render(
      <AgentActivityModule agents={agents} statusText={null} activities={[]} toolCallCount={0} />,
    )
    expect(container.querySelector('.activity-module__status')).not.toBeInTheDocument()
  })

  it('renders tool call count', () => {
    render(
      <AgentActivityModule agents={[]} statusText={null} activities={activities} toolCallCount={4} />,
    )
    expect(screen.getByText('4 tool calls')).toBeInTheDocument()
  })

  it('renders singular tool call count', () => {
    render(
      <AgentActivityModule
        agents={[]}
        statusText={null}
        activities={activities.slice(0, 1)}
        toolCallCount={1}
      />,
    )
    expect(screen.getByText('1 tool call')).toBeInTheDocument()
  })

  it('renders activity feed lines', () => {
    render(
      <AgentActivityModule agents={[]} statusText={null} activities={activities} toolCallCount={2} />,
    )
    expect(screen.getByText('search_files')).toBeInTheDocument()
    expect(screen.getByText('read_file')).toBeInTheDocument()
  })

  it('shows checkmark for completed lines', () => {
    const { container } = render(
      <AgentActivityModule agents={[]} statusText={null} activities={activities} toolCallCount={2} />,
    )
    const completedIndicator = container.querySelector(
      '.activity-module__line--completed .activity-module__line-indicator',
    )
    expect(completedIndicator?.textContent).toBe('\u2713')
  })

  it('shows spinner for running lines', () => {
    const { container } = render(
      <AgentActivityModule agents={[]} statusText={null} activities={activities} toolCallCount={2} />,
    )
    const runningIndicator = container.querySelector(
      '.activity-module__line--running .activity-module__line-indicator',
    )
    expect(runningIndicator?.textContent).toBe('\u27F3')
  })

  it('renders empty wrappers when no data', () => {
    const { container } = render(
      <AgentActivityModule agents={[]} statusText={null} activities={[]} toolCallCount={0} />,
    )
    expect(container.querySelector('.activity-module')).toBeInTheDocument()
    expect(container.querySelector('.activity-module__agents')?.children.length).toBe(0)
    expect(container.querySelector('.activity-module__feed')?.children.length).toBe(0)
  })
})
