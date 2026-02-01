import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PipelineStageNode } from './PipelineStageNode'

const defaults = {
  stageNumber: 1,
  stageName: 'decompose',
  status: 'pending' as const,
  approvalRequired: false,
  agentName: null,
  durationMs: null,
  tokenCount: null,
  isCurrent: false,
}

describe('PipelineStageNode', () => {
  it('renders stage number and name', () => {
    render(<PipelineStageNode {...defaults} />)
    expect(screen.getByText(/1: decompose/)).toBeInTheDocument()
  })

  it('shows check for completed status', () => {
    render(<PipelineStageNode {...defaults} status="completed" />)
    expect(screen.getByText('\u2713')).toBeInTheDocument()
  })

  it('shows X for failed status', () => {
    render(<PipelineStageNode {...defaults} status="failed" />)
    expect(screen.getByText('\u2717')).toBeInTheDocument()
  })

  it('shows approval gate icon when required', () => {
    render(<PipelineStageNode {...defaults} approvalRequired={true} />)
    expect(screen.getByText('\u2298')).toBeInTheDocument()
  })

  it('renders agent name when provided', () => {
    render(<PipelineStageNode {...defaults} agentName="planner" />)
    expect(screen.getByText('planner')).toBeInTheDocument()
  })

  it('formats duration in seconds', () => {
    render(<PipelineStageNode {...defaults} durationMs={2300} />)
    expect(screen.getByText('2.3s')).toBeInTheDocument()
  })

  it('formats duration in ms when under 1s', () => {
    render(<PipelineStageNode {...defaults} durationMs={450} />)
    expect(screen.getByText('450ms')).toBeInTheDocument()
  })

  it('formats token count with k suffix', () => {
    render(<PipelineStageNode {...defaults} tokenCount={1500} />)
    expect(screen.getByText('1.5k tok')).toBeInTheDocument()
  })

  it('applies current modifier when isCurrent', () => {
    const { container } = render(<PipelineStageNode {...defaults} isCurrent={true} />)
    expect(container.querySelector('.stage-node--current')).toBeInTheDocument()
  })

  it('applies pending modifier when not current', () => {
    const { container } = render(<PipelineStageNode {...defaults} />)
    expect(container.querySelector('.stage-node--pending')).toBeInTheDocument()
  })

  it('hides meta row when no agent, duration, or tokens', () => {
    const { container } = render(<PipelineStageNode {...defaults} />)
    expect(container.querySelector('.stage-node__meta')).not.toBeInTheDocument()
  })
})
