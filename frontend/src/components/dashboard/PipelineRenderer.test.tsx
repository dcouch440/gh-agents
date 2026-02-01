import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PipelineRenderer } from './PipelineRenderer'
import type { Pipeline, PipelineRun, StageExecution } from '@/types'

const pipeline: Pipeline = {
  id: 'p1',
  name: 'deploy',
  stages: [
    { stage_number: 1, agent_id: null, cluster_id: null, role: null, approval_required: false, fan_out: false, stage_name: 'plan', input_definitions: {}, output_description: '', output_schema: null },
    { stage_number: 2, agent_id: null, cluster_id: null, role: null, approval_required: true, fan_out: false, stage_name: 'execute', input_definitions: {}, output_description: '', output_schema: null },
  ],
}

const run: PipelineRun = {
  id: 'r1',
  pipeline_id: 'p1',
  user_id: 'u1',
  status: 'running',
  initial_task: 'deploy app',
  stage_outputs: {},
  current_stage: 1,
  started_at: new Date().toISOString(),
  completed_at: null,
  total_input_tokens: 2400,
  total_output_tokens: 800,
}

const stages: StageExecution[] = [
  { id: 's1', run_id: 'r1', stage_number: 1, stage_name: 'plan', agent_id: 'a1', status: 'running', rendered_prompt: null, output: null, structured_output: null, user_input: null, input_tokens: 2400, output_tokens: 800, started_at: new Date().toISOString(), completed_at: null, duration_ms: 1500 },
]

describe('PipelineRenderer', () => {
  it('shows empty state when no run', () => {
    render(<PipelineRenderer pipeline={pipeline} run={null} stages={[]} />)
    expect(screen.getByText('no active run')).toBeInTheDocument()
  })

  it('renders pipeline name and status', () => {
    render(<PipelineRenderer pipeline={pipeline} run={run} stages={stages} />)
    expect(screen.getByText(/PIPELINE: deploy/)).toBeInTheDocument()
    expect(screen.getByText(/RUNNING/)).toBeInTheDocument()
  })

  it('renders stage count', () => {
    render(<PipelineRenderer pipeline={pipeline} run={run} stages={stages} />)
    expect(screen.getByText(/STAGE: 1\/2/)).toBeInTheDocument()
  })

  it('renders token totals', () => {
    render(<PipelineRenderer pipeline={pipeline} run={run} stages={stages} />)
    expect(screen.getByText(/2\.4k/)).toBeInTheDocument()
    expect(screen.getByText(/800/)).toBeInTheDocument()
  })

  it('renders stage nodes', () => {
    render(<PipelineRenderer pipeline={pipeline} run={run} stages={stages} />)
    expect(screen.getByText(/1: plan/)).toBeInTheDocument()
    expect(screen.getByText(/2: execute/)).toBeInTheDocument()
  })

  it('shows approval connector for approval stages', () => {
    const { container } = render(<PipelineRenderer pipeline={pipeline} run={run} stages={stages} />)
    const connectors = container.querySelectorAll('.pipeline__connector')
    expect(connectors.length).toBe(1)
    expect(connectors[0]?.textContent).toContain('\u2298')
  })
})
