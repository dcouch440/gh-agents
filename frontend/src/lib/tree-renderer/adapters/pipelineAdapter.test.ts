import { describe, it, expect } from 'vitest'
import { pipelineToTree } from './pipelineAdapter'
import type { Pipeline, PipelineRun, StageExecution } from '@/types'

const createMockPipeline = (overrides?: Partial<Pipeline>): Pipeline => ({
  id: 'pipeline-001',
  name: 'Test pipeline',
  stages: [
    { stage_number: 1, stage_name: 'Planning', agent_tier: 'orchestrator', approval_required: false },
    { stage_number: 2, stage_name: 'Execution', agent_tier: 'worker', approval_required: false },
    { stage_number: 3, stage_name: 'Review', agent_tier: 'utility', approval_required: true },
  ],
  ...overrides,
})

const createMockRun = (overrides?: Partial<PipelineRun>): PipelineRun => ({
  id: 'run-001',
  pipeline_id: 'pipeline-001',
  user_id: 'user-001',
  status: 'running',
  initial_task: 'Do the thing',
  stage_outputs: {},
  current_stage: 1,
  started_at: '2025-01-01T00:00:00Z',
  completed_at: null,
  total_input_tokens: 0,
  total_output_tokens: 0,
  ...overrides,
})

const createMockExecution = (overrides?: Partial<StageExecution>): StageExecution => ({
  id: 'exec-001',
  run_id: 'run-001',
  stage_number: 1,
  stage_name: 'Planning',
  agent_id: 'agent-001',
  status: 'completed',
  rendered_prompt: 'Plan the task',
  output: 'Here is the plan',
  structured_output: null,
  user_input: null,
  input_tokens: 500,
  output_tokens: 200,
  started_at: '2025-01-01T00:00:00Z',
  completed_at: '2025-01-01T00:00:01Z',
  duration_ms: 1000,
  ...overrides,
})

describe('pipelineToTree', () => {
  it('converts pipeline with no run to pending nodes', () => {
    const pipeline = createMockPipeline()
    const result = pipelineToTree(pipeline, null, [])

    expect(result.rootIds).toEqual(['stage-1'])
    expect(result.nodes['stage-1']).toMatchObject({
      id: 'stage-1',
      label: 'Planning',
      status: 'pending',
      children: ['stage-2'],
    })
    expect(result.nodes['stage-2']).toMatchObject({
      id: 'stage-2',
      label: 'Execution',
      status: 'pending',
      children: ['stage-3'],
    })
    expect(result.nodes['stage-3']).toMatchObject({
      id: 'stage-3',
      label: 'Review',
      status: 'pending',
      children: [],
    })
  })

  it('creates edges between consecutive stages', () => {
    const pipeline = createMockPipeline()
    const result = pipelineToTree(pipeline, null, [])

    expect(result.edges).toEqual([
      {
        sourceId: 'stage-1',
        targetId: 'stage-2',
        label: null,
        variant: 'normal',
      },
      {
        sourceId: 'stage-2',
        targetId: 'stage-3',
        label: null,
        variant: 'approval',
      },
    ])
  })

  it('marks approval edges based on stage configuration', () => {
    const pipeline = createMockPipeline()
    const result = pipelineToTree(pipeline, null, [])

    const approvalEdge = result.edges.find((e) => e.targetId === 'stage-3')
    expect(approvalEdge?.variant).toBe('approval')

    const normalEdge = result.edges.find((e) => e.targetId === 'stage-2')
    expect(normalEdge?.variant).toBe('normal')
  })

  it('sets status to running for current stage', () => {
    const pipeline = createMockPipeline()
    const run = createMockRun({ current_stage: 2, status: 'running' })
    const executions = [
      createMockExecution({ stage_number: 1, status: 'completed' }),
      createMockExecution({ stage_number: 2, status: 'running' }),
    ]
    const result = pipelineToTree(pipeline, run, executions)

    expect(result.nodes['stage-1']?.status).toBe('completed')
    expect(result.nodes['stage-2']?.status).toBe('running')
    expect(result.nodes['stage-3']?.status).toBe('pending')
  })

  it('sets status to waiting when waiting for approval', () => {
    const pipeline = createMockPipeline()
    const run = createMockRun({ current_stage: 3, status: 'waiting_for_approval' })
    const executions = [
      createMockExecution({ stage_number: 1, status: 'completed' }),
      createMockExecution({ stage_number: 2, status: 'completed' }),
      createMockExecution({ stage_number: 3, status: 'running' }),
    ]
    const result = pipelineToTree(pipeline, run, executions)

    expect(result.nodes['stage-3']?.status).toBe('waiting')
  })

  it('sets status to failed when execution fails', () => {
    const pipeline = createMockPipeline()
    const run = createMockRun({ current_stage: 2 })
    const executions = [
      createMockExecution({ stage_number: 1, status: 'completed' }),
      createMockExecution({ stage_number: 2, status: 'failed' }),
    ]
    const result = pipelineToTree(pipeline, run, executions)

    expect(result.nodes['stage-2']?.status).toBe('failed')
  })

  it('preserves stage metadata from execution', () => {
    const pipeline = createMockPipeline()
    const run = createMockRun()
    const executions = [
      createMockExecution({
        stage_number: 1,
        agent_id: 'agent-123',
        duration_ms: 5000,
        input_tokens: 1000,
        output_tokens: 500,
      }),
    ]
    const result = pipelineToTree(pipeline, run, executions)

    expect(result.nodes['stage-1']?.metadata).toEqual({
      stageNumber: 1,
      approvalRequired: false,
      agentId: 'agent-123',
      durationMs: 5000,
      inputTokens: 1000,
      outputTokens: 500,
    })
  })

  it('sets null metadata when no execution exists', () => {
    const pipeline = createMockPipeline()
    const result = pipelineToTree(pipeline, null, [])

    expect(result.nodes['stage-1']?.metadata).toEqual({
      stageNumber: 1,
      approvalRequired: false,
      agentId: null,
      durationMs: null,
      inputTokens: 0,
      outputTokens: 0,
    })
  })

  it('handles single-stage pipeline', () => {
    const pipeline: Pipeline = {
      id: 'pipeline-001',
      name: 'Single stage',
      stages: [
        { stage_number: 1, stage_name: 'Only Stage', agent_tier: 'worker', approval_required: false },
      ],
    }
    const result = pipelineToTree(pipeline, null, [])

    expect(result.rootIds).toEqual(['stage-1'])
    expect(result.nodes['stage-1']?.children).toEqual([])
    expect(result.edges).toEqual([])
  })

  it('handles empty pipeline', () => {
    const pipeline: Pipeline = {
      id: 'pipeline-001',
      name: 'Empty',
      stages: [],
    }
    const result = pipelineToTree(pipeline, null, [])

    expect(result.rootIds).toEqual([])
    expect(result.nodes).toEqual({})
    expect(result.edges).toEqual([])
  })

  it('correctly derives status for completed execution', () => {
    const pipeline = createMockPipeline()
    const run = createMockRun({ current_stage: 3 })
    const executions = [
      createMockExecution({ stage_number: 1, status: 'completed' }),
      createMockExecution({ stage_number: 2, status: 'completed' }),
    ]
    const result = pipelineToTree(pipeline, run, executions)

    expect(result.nodes['stage-1']?.status).toBe('completed')
    expect(result.nodes['stage-2']?.status).toBe('completed')
  })
})
