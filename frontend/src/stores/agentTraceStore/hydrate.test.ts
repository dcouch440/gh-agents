import { describe, it, expect, vi, beforeEach } from 'vitest'
import { agentTraceStore } from '.'
import { hydrateFromTimeline, setHydratedRun, parseToolInput } from './hydrate'
import type { AgentTraceEvent } from '.'
import { Collections } from '@/utils/collections'
import type { TimelineEntry } from '@/types'
import type * as ApiModule from '@/api'

const { mockGetExecutionTimeline } = vi.hoisted(() => ({
  mockGetExecutionTimeline: vi.fn(),
}))

vi.mock('@/api', async (importOriginal) => ({
  ...(await importOriginal<typeof ApiModule>()),
  api: {
    workflows: {
      getExecutionTimeline: mockGetExecutionTimeline,
    },
  },
}))

const makeEntry = (overrides: Partial<TimelineEntry> = {}): TimelineEntry => ({
  id: 'e1',
  ts: '2025-01-01T00:00:01Z',
  kind: 'assistant_message',
  step_id: 'step-1',
  step_name: 'Research',
  agent_name: 'Scanner',
  agent_execution_id: 'ae-1',
  content: 'hello',
  tool_name: null,
  tool_call_id: null,
  input_tokens: 10,
  output_tokens: 5,
  ...overrides,
})

beforeEach(() => {
  vi.clearAllMocks()
  agentTraceStore.reset()
  mockGetExecutionTimeline.mockResolvedValue({ entries: [], has_more: false, next_cursor: null })
})

describe('hydrateFromTimeline', () => {
  it('groups entries by agent execution', async () => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [
        makeEntry({ id: 'e1', agent_execution_id: 'ae-1', content: 'first' }),
        makeEntry({ id: 'e2', agent_execution_id: 'ae-2', agent_name: 'Writer', content: 'second' }),
        makeEntry({ id: 'e3', agent_execution_id: 'ae-1', content: 'third' }),
      ],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    const s = agentTraceStore.store.getState()
    expect(s.order).toEqual(['ae-1', 'ae-2'])
    expect(s.traces['ae-1']?.events).toHaveLength(2)
    expect(s.traces['ae-2']?.agentName).toBe('Writer')
  })

  it('carries step_id through so traces can be grouped by node', async () => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [makeEntry({ step_id: 'step-9' })],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    expect(agentTraceStore.store.getState().traces['ae-1']?.stepId).toBe('step-9')
  })

  it.each([
    ['system_prompt', 'system_prompt'],
    ['user_message', 'user_message'],
    ['assistant_message', 'assistant_message'],
  ] as const)('maps %s onto the matching event type', async (kind, expected) => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [makeEntry({ kind })],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    expect(agentTraceStore.store.getState().traces['ae-1']?.events[0]?.type).toBe(expected)
  })

  it('parses a JSON tool_call payload into structured input', async () => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [makeEntry({ kind: 'tool_call', tool_name: 'search', tool_call_id: 't1', content: '{"query":"rust"}' })],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    const event = agentTraceStore.store.getState().traces['ae-1']?.events[0]
    expect(event?.type).toBe('tool_call')
    if (event?.type === 'tool_call') {
      expect(event.input).toEqual({ query: 'rust' })
      expect(event.toolName).toBe('search')
    }
  })

  it('never clobbers a richer live trace', async () => {
    agentTraceStore.store.setState({
      traces: { 'ae-1': { agentExecutionId: 'ae-1', agentName: 'Live', stepId: 'step-1', events: [] } },
      order: ['ae-1'],
    })
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [makeEntry({ agent_name: 'FromDb' })],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    expect(agentTraceStore.store.getState().traces['ae-1']?.agentName).toBe('Live')
    expect(agentTraceStore.store.getState().order).toEqual(['ae-1'])
  })

  it('swallows a failed fetch', async () => {
    mockGetExecutionTimeline.mockRejectedValue(new Error('boom'))

    await hydrateFromTimeline('run-1')

    expect(agentTraceStore.store.getState().order).toEqual([])
  })
})

describe('tool call/result pairing', () => {
  /** Tool ids for one event kind, in order. */
  const toolIds = (events: readonly AgentTraceEvent[], type: 'tool_call' | 'tool_result'): string[] =>
    Collections.filterMap(events, (e) =>
      e.type === 'tool_call' || e.type === 'tool_result'
        ? (e.type === type ? e.toolId : null)
        : null,
    )

  const toolIdOf = (events: readonly AgentTraceEvent[], type: 'tool_call' | 'tool_result'): string => {
    const ids = toolIds(events, type)
    if (ids[0] === undefined) throw new Error(`no ${type} event`)
    return ids[0]
  }

  it('pairs a call with its result even though the DB links neither', async () => {
    // execution_messages stores tool_call_id only on the tool-result row; the
    // assistant row that issued the call has none. Falling back to each row's
    // own id left every tool block rendering as perpetually running.
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [
        makeEntry({ id: 'm1', kind: 'tool_call', tool_name: 'run_command', tool_call_id: null, content: '{}' }),
        makeEntry({ id: 'm2', kind: 'tool_result', tool_name: 'run_command', tool_call_id: 'toolu_abc', content: 'ok' }),
      ],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    const events = agentTraceStore.store.getState().traces['ae-1']?.events ?? []
    expect(toolIdOf(events, 'tool_call')).toBe(toolIdOf(events, 'tool_result'))
  })

  it('pairs multiple calls in order within one agent execution', async () => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [
        makeEntry({ id: 'c1', kind: 'tool_call', tool_call_id: null, content: '{}' }),
        makeEntry({ id: 'r1', kind: 'tool_result', tool_call_id: 't1', content: 'first' }),
        makeEntry({ id: 'c2', kind: 'tool_call', tool_call_id: null, content: '{}' }),
        makeEntry({ id: 'r2', kind: 'tool_result', tool_call_id: 't2', content: 'second' }),
      ],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    const events = agentTraceStore.store.getState().traces['ae-1']?.events ?? []
    const calls = toolIds(events, 'tool_call')
    const results = toolIds(events, 'tool_result')

    // Each call matches its own result, and the two pairs stay distinct.
    expect(calls).toEqual(results)
    expect(calls[0]).not.toBe(calls[1])
  })

  it('keys agent executions separately so their tools never cross-pair', async () => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [
        makeEntry({ id: 'a', agent_execution_id: 'ae-1', kind: 'tool_call', tool_call_id: null, content: '{}' }),
        makeEntry({ id: 'b', agent_execution_id: 'ae-2', kind: 'tool_result', tool_call_id: 't1', content: 'ok' }),
      ],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    const s = agentTraceStore.store.getState()
    const call = toolIdOf(s.traces['ae-1']?.events ?? [], 'tool_call')
    const result = toolIdOf(s.traces['ae-2']?.events ?? [], 'tool_result')
    expect(call).not.toBe(result)
  })

  it('leaves a call with no result unpaired, so it correctly reads as running', async () => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [
        makeEntry({ id: 'c1', kind: 'tool_call', tool_call_id: null, content: '{}' }),
        makeEntry({ id: 'r1', kind: 'tool_result', tool_call_id: 't1', content: 'first' }),
        makeEntry({ id: 'c2', kind: 'tool_call', tool_call_id: null, content: '{}' }),
      ],
      has_more: false,
      next_cursor: null,
    })

    await hydrateFromTimeline('run-1')

    const events = agentTraceStore.store.getState().traces['ae-1']?.events ?? []
    const resultIds = Collections.toSet(toolIds(events, 'tool_result'))
    const unmatched = toolIds(events, 'tool_call').filter((id) => !resultIds.has(id))
    expect(unmatched).toHaveLength(1)
  })
})

describe('parseToolInput', () => {
  it('keeps unparseable content as raw text', () => {
    expect(parseToolInput('not json')).toEqual({ raw: 'not json' })
  })

  it('rejects a JSON array, which is not an input object', () => {
    expect(parseToolInput('[1,2]')).toEqual({ raw: '[1,2]' })
  })
})

describe('setHydratedRun', () => {
  it('records the run and clears traces when the run changes', () => {
    agentTraceStore.store.setState({
      traces: { 'ae-1': { agentExecutionId: 'ae-1', agentName: 'A', stepId: 's1', events: [] } },
      order: ['ae-1'],
      hydratedRunId: 'run-1',
    })

    setHydratedRun('run-2')

    const s = agentTraceStore.store.getState()
    expect(s.hydratedRunId).toBe('run-2')
    expect(s.order).toEqual([])
  })

  it('leaves traces alone when the run is unchanged', () => {
    agentTraceStore.store.setState({
      traces: { 'ae-1': { agentExecutionId: 'ae-1', agentName: 'A', stepId: 's1', events: [] } },
      order: ['ae-1'],
      hydratedRunId: 'run-1',
    })

    setHydratedRun('run-1')

    expect(agentTraceStore.store.getState().order).toEqual(['ae-1'])
  })
})
