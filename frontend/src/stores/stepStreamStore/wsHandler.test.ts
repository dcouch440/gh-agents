import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { WsWireMessage } from '@/types/ws'

// Must import after vitest setup
const { stepStreamStore } = await import('.')

const wire = (event: string, data: Record<string, unknown>, ts = '2025-01-01T00:00:00Z'): WsWireMessage => ({
  topic: 'workflow',
  event,
  ts,
  run_id: 'run-1',
  user_id: 'user-1',
  seq: 1,
  data,
})

describe('stepStreamStore wsHandler', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Reset store
    stepStreamStore.handleWsEvent(wire('started', { workflow_id: 'wf-1', total_steps: 3 }))
  })

  describe('workflow started', () => {
    it('resets entire store', () => {
      // Seed some state
      stepStreamStore.handleWsEvent(wire('workforce_designer_progress', { workflow_id: 'wf-1', step_id: 's-1', status: 'started' }))
      expect(stepStreamStore.store.getState().designerStatus).toBe('running')

      // Reset
      stepStreamStore.handleWsEvent(wire('started', { workflow_id: 'wf-1', total_steps: 2 }))
      const state = stepStreamStore.store.getState()
      expect(state.designerStatus).toBe('idle')
      expect(state.sources).toEqual({})
      expect(state.activeStepId).toBeNull()
    })
  })

  describe('workforce_designer_progress', () => {
    it('tracks designer started', () => {
      stepStreamStore.handleWsEvent(wire('workforce_designer_progress', { workflow_id: 'wf-1', step_id: 's-1', status: 'started' }))
      expect(stepStreamStore.store.getState().designerStatus).toBe('running')
      expect(stepStreamStore.store.getState().activeStepId).toBe('s-1')
    })

    it('tracks designer completed', () => {
      stepStreamStore.handleWsEvent(wire('workforce_designer_progress', { workflow_id: 'wf-1', step_id: 's-1', status: 'completed' }))
      expect(stepStreamStore.store.getState().designerStatus).toBe('completed')
    })

    it('tracks designer failed', () => {
      stepStreamStore.handleWsEvent(wire('workforce_designer_progress', { workflow_id: 'wf-1', step_id: 's-1', status: 'failed' }))
      expect(stepStreamStore.store.getState().designerStatus).toBe('failed')
    })
  })

  describe('workforce_agent_progress', () => {
    it('creates source entry on started', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Researcher', roster_agent_id: 'ra-1',
        agent_index: 0, total_agents: 3, status: 'started',
      }, '2025-01-01T00:01:00Z'))

      const src = stepStreamStore.store.getState().sources['ra-1']
      expect(src).toBeDefined()
      expect(src!.status).toBe('running')
      expect(src!.sourceName).toBe('Researcher')
      expect(src!.startedAt).toBe('2025-01-01T00:01:00Z')
    })

    it('updates source on completed', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Researcher', roster_agent_id: 'ra-1',
        agent_index: 0, total_agents: 3, status: 'started',
      }))
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Researcher', roster_agent_id: 'ra-1',
        agent_index: 0, total_agents: 3, status: 'completed',
      }, '2025-01-01T00:05:00Z'))

      const src = stepStreamStore.store.getState().sources['ra-1']
      expect(src!.status).toBe('completed')
      expect(src!.completedAt).toBe('2025-01-01T00:05:00Z')
    })

    it('updates source on failed', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Researcher', roster_agent_id: 'ra-1',
        agent_index: 0, total_agents: 3, status: 'started',
      }))
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Researcher', roster_agent_id: 'ra-1',
        agent_index: 0, total_agents: 3, status: 'failed',
      }))

      expect(stepStreamStore.store.getState().sources['ra-1']!.status).toBe('failed')
    })
  })

  describe('step_stream_token', () => {
    it('appends tokens to stream buffer', () => {
      // Create source first
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Writer', roster_agent_id: 'ra-2',
        agent_index: 0, total_agents: 1, status: 'started',
      }))

      stepStreamStore.handleWsEvent(wire('step_stream_token', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'ra-2', source_name: 'Writer', content: 'Hello ',
      }))
      stepStreamStore.handleWsEvent(wire('step_stream_token', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'ra-2', source_name: 'Writer', content: 'world!',
      }))

      expect(stepStreamStore.store.getState().sources['ra-2']!.streamBuffer).toBe('Hello world!')
    })

    it('auto-creates source if not present', () => {
      stepStreamStore.handleWsEvent(wire('step_stream_token', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'new-src', source_name: 'Auto', content: 'hi',
      }))

      const src = stepStreamStore.store.getState().sources['new-src']
      expect(src).toBeDefined()
      expect(src!.sourceName).toBe('Auto')
      expect(src!.status).toBe('running')
      expect(src!.streamBuffer).toBe('hi')
    })
  })

  describe('step_stream_tool_start / step_stream_tool_end', () => {
    it('tracks tool lifecycle', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Dev', roster_agent_id: 'ra-3',
        agent_index: 0, total_agents: 1, status: 'started',
      }))

      stepStreamStore.handleWsEvent(wire('step_stream_tool_start', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'ra-3', source_name: 'Dev',
        tool_name: 'write_file', tool_id: 'tool-1',
      }, '2025-01-01T00:02:00Z'))

      const tools1 = stepStreamStore.store.getState().sources['ra-3']!.toolUses
      expect(tools1).toHaveLength(1)
      expect(tools1[0]!.toolName).toBe('write_file')
      expect(tools1[0]!.status).toBe('running')

      stepStreamStore.handleWsEvent(wire('step_stream_tool_end', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'ra-3', source_name: 'Dev',
        tool_name: 'write_file', tool_id: 'tool-1',
      }))

      const tools2 = stepStreamStore.store.getState().sources['ra-3']!.toolUses
      expect(tools2[0]!.status).toBe('completed')
    })

    it('ignores tool events for unknown sources', () => {
      stepStreamStore.handleWsEvent(wire('step_stream_tool_start', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'unknown', source_name: 'X',
        tool_name: 'test', tool_id: 't-1',
      }))

      expect(stepStreamStore.store.getState().sources['unknown']).toBeUndefined()
    })
  })

  describe('step_stream_error', () => {
    it('sets error on source', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Bot', roster_agent_id: 'ra-4',
        agent_index: 0, total_agents: 1, status: 'started',
      }))

      stepStreamStore.handleWsEvent(wire('step_stream_error', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'ra-4', source_name: 'Bot',
        error: 'Rate limit exceeded',
      }))

      expect(stepStreamStore.store.getState().sources['ra-4']!.error).toBe('Rate limit exceeded')
    })
  })

  describe('multiple sources', () => {
    it('maintains independent state per source', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Alpha', roster_agent_id: 'ra-a',
        agent_index: 0, total_agents: 2, status: 'started',
      }))
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Beta', roster_agent_id: 'ra-b',
        agent_index: 1, total_agents: 2, status: 'started',
      }))

      stepStreamStore.handleWsEvent(wire('step_stream_token', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'ra-a', source_name: 'Alpha', content: 'AAA',
      }))
      stepStreamStore.handleWsEvent(wire('step_stream_token', {
        workflow_id: 'wf-1', step_id: 's-1', source_id: 'ra-b', source_name: 'Beta', content: 'BBB',
      }))

      const state = stepStreamStore.store.getState()
      expect(state.sources['ra-a']!.streamBuffer).toBe('AAA')
      expect(state.sources['ra-b']!.streamBuffer).toBe('BBB')
    })
  })

  describe('designer_agent_designed', () => {
    it('tracks per-agent design progress', () => {
      stepStreamStore.handleWsEvent(wire('designer_agent_designed', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'scanner', designed_count: 1, total_count: 3,
      }))

      const state = stepStreamStore.store.getState()
      const ds = state.designStatusByStep['s-1']
      expect(ds).toBeDefined()
      expect(ds!.status).toBe('running')
      expect(ds!.designedCount).toBe(1)
      expect(ds!.totalCount).toBe(3)
      expect(ds!.lastAgentName).toBe('scanner')
    })

    it('increments designed count across agents', () => {
      stepStreamStore.handleWsEvent(wire('designer_agent_designed', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'scanner', designed_count: 1, total_count: 3,
      }))
      stepStreamStore.handleWsEvent(wire('designer_agent_designed', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'writer', designed_count: 2, total_count: 3,
      }))

      const ds = stepStreamStore.store.getState().designStatusByStep['s-1']
      expect(ds!.designedCount).toBe(2)
      expect(ds!.lastAgentName).toBe('writer')
    })

    it('tracks separate steps independently', () => {
      stepStreamStore.handleWsEvent(wire('designer_agent_designed', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'a1', designed_count: 1, total_count: 2,
      }))
      stepStreamStore.handleWsEvent(wire('designer_agent_designed', {
        workflow_id: 'wf-1', step_id: 's-2',
        agent_name: 'b1', designed_count: 1, total_count: 4,
      }))

      const state = stepStreamStore.store.getState()
      expect(state.designStatusByStep['s-1']!.totalCount).toBe(2)
      expect(state.designStatusByStep['s-2']!.totalCount).toBe(4)
    })
  })

  describe('designStatusByStep via workforce_designer_progress', () => {
    it('sets design status to running on started', () => {
      stepStreamStore.handleWsEvent(wire('workforce_designer_progress', {
        workflow_id: 'wf-1', step_id: 's-1', status: 'started',
      }))

      const ds = stepStreamStore.store.getState().designStatusByStep['s-1']
      expect(ds).toBeDefined()
      expect(ds!.status).toBe('running')
    })

    it('sets design status to completed', () => {
      // Start, then add agent progress, then complete
      stepStreamStore.handleWsEvent(wire('designer_agent_designed', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'a', designed_count: 2, total_count: 2,
      }))
      stepStreamStore.handleWsEvent(wire('workforce_designer_progress', {
        workflow_id: 'wf-1', step_id: 's-1', status: 'completed',
      }))

      const ds = stepStreamStore.store.getState().designStatusByStep['s-1']
      expect(ds!.status).toBe('completed')
      expect(ds!.designedCount).toBe(2)
    })

    it('sets design status to failed', () => {
      stepStreamStore.handleWsEvent(wire('workforce_designer_progress', {
        workflow_id: 'wf-1', step_id: 's-1', status: 'failed',
      }))

      expect(stepStreamStore.store.getState().designStatusByStep['s-1']!.status).toBe('failed')
    })
  })

  describe('selectDesignStatusForStep', () => {
    it('returns null when no design state exists', () => {
      const state = stepStreamStore.store.getState()
      expect(stepStreamStore.selectDesignStatusForStep('unknown')(state)).toBeNull()
    })

    it('returns design state for tracked step', () => {
      stepStreamStore.handleWsEvent(wire('designer_agent_designed', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'test', designed_count: 1, total_count: 3,
      }))

      const state = stepStreamStore.store.getState()
      const ds = stepStreamStore.selectDesignStatusForStep('s-1')(state)
      expect(ds).not.toBeNull()
      expect(ds!.status).toBe('running')
      expect(ds!.designedCount).toBe(1)
    })
  })

  describe('selectors', () => {
    it('selectSource returns specific source', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'Test', roster_agent_id: 'ra-sel',
        agent_index: 0, total_agents: 1, status: 'started',
      }))

      const state = stepStreamStore.store.getState()
      const src = stepStreamStore.selectSource('ra-sel')(state)
      expect(src).not.toBeNull()
      expect(src!.sourceName).toBe('Test')
    })

    it('selectSource returns null for unknown', () => {
      const state = stepStreamStore.store.getState()
      expect(stepStreamStore.selectSource('nope')(state)).toBeNull()
    })

    it('selectSourcesForStep filters by step', () => {
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-1',
        agent_name: 'A', roster_agent_id: 'ra-x',
        agent_index: 0, total_agents: 1, status: 'started',
      }))
      stepStreamStore.handleWsEvent(wire('workforce_agent_progress', {
        workflow_id: 'wf-1', step_id: 's-2',
        agent_name: 'B', roster_agent_id: 'ra-y',
        agent_index: 0, total_agents: 1, status: 'started',
      }))

      const state = stepStreamStore.store.getState()
      const step1Sources = stepStreamStore.selectSourcesForStep('s-1')(state)
      expect(step1Sources).toHaveLength(1)
      expect(step1Sources[0]!.sourceId).toBe('ra-x')
    })
  })
})
