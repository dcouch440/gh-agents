import { describe, it, expect, beforeEach } from 'vitest'
import type { WsWireMessage } from '@/types/ws'

const { agentTraceStore } = await import('.')

const wire = (event: string, data: Record<string, unknown>, ts = '2025-01-01T00:00:00Z'): WsWireMessage => ({
  topic: 'workflow',
  event,
  ts,
  run_id: 'run-1',
  user_id: 'user-1',
  seq: 1,
  data,
})

describe('agentTraceStore wsHandler', () => {
  beforeEach(() => {
    agentTraceStore.reset()
  })

  describe('workflow started', () => {
    it('resets all traces', () => {
      agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Agent', content: 'prompt',
      }))
      expect(agentTraceStore.store.getState().order).toHaveLength(1)

      agentTraceStore.handleWsEvent(wire('started', { workflow_id: 'wf-1', total_steps: 2 }))
      const state = agentTraceStore.store.getState()
      expect(state.traces).toEqual({})
      expect(state.order).toEqual([])
    })
  })

  describe('debug_system_prompt', () => {
    it('creates trace with system prompt event', () => {
      agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Researcher', content: 'You are a researcher.',
      }, '2025-01-01T00:01:00Z'))

      const state = agentTraceStore.store.getState()
      expect(state.order).toEqual(['ae-1'])
      const trace = state.traces['ae-1']
      expect(trace).toBeDefined()
      expect(trace!.agentName).toBe('Researcher')
      expect(trace!.stepId).toBe('s-1')
      expect(trace!.events).toHaveLength(1)
      expect(trace!.events[0]).toEqual({
        type: 'system_prompt',
        content: 'You are a researcher.',
        ts: '2025-01-01T00:01:00Z',
      })
    })
  })

  describe('debug_user_message', () => {
    it('appends user message to existing trace', () => {
      agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Bot', content: 'system',
      }))
      agentTraceStore.handleWsEvent(wire('debug_user_message', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Bot', content: 'Do something',
      }))

      const trace = agentTraceStore.store.getState().traces['ae-1']
      expect(trace!.events).toHaveLength(2)
      expect(trace!.events[1]!.type).toBe('user_message')
    })
  })

  describe('debug_assistant_message', () => {
    it('appends assistant message', () => {
      agentTraceStore.handleWsEvent(wire('debug_assistant_message', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Bot', content: 'Here is my response.',
      }))

      const trace = agentTraceStore.store.getState().traces['ae-1']
      expect(trace!.events[0]).toEqual(expect.objectContaining({ type: 'assistant_message', content: 'Here is my response.' }))
    })
  })

  describe('debug_tool_call', () => {
    it('stores tool call with input', () => {
      agentTraceStore.handleWsEvent(wire('debug_tool_call', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Bot',
        tool_name: 'search', tool_id: 't-1', input: { query: 'test' },
      }))

      const event = agentTraceStore.store.getState().traces['ae-1']!.events[0]!
      expect(event.type).toBe('tool_call')
      if (event.type === 'tool_call') {
        expect(event.toolName).toBe('search')
        expect(event.input).toEqual({ query: 'test' })
      }
    })
  })

  describe('debug_tool_result', () => {
    it('stores tool result', () => {
      agentTraceStore.handleWsEvent(wire('debug_tool_result', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Bot',
        tool_name: 'search', tool_id: 't-1', result: 'Found 3 results',
      }))

      const event = agentTraceStore.store.getState().traces['ae-1']!.events[0]!
      expect(event.type).toBe('tool_result')
      if (event.type === 'tool_result') {
        expect(event.result).toBe('Found 3 results')
      }
    })
  })

  describe('multiple agents', () => {
    it('groups events by agent_execution_id and preserves order', () => {
      agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Alpha', content: 'p1',
      }))
      agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-2', agent_name: 'Beta', content: 'p2',
      }))
      agentTraceStore.handleWsEvent(wire('debug_user_message', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'Alpha', content: 'msg1',
      }))

      const state = agentTraceStore.store.getState()
      expect(state.order).toEqual(['ae-1', 'ae-2'])
      expect(state.traces['ae-1']!.events).toHaveLength(2)
      expect(state.traces['ae-2']!.events).toHaveLength(1)
    })
  })

  describe('null agent name', () => {
    it('handles null agent_name', () => {
      agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: null, content: 'prompt',
      }))

      expect(agentTraceStore.store.getState().traces['ae-1']!.agentName).toBeNull()
    })
  })

  describe('selectors', () => {
    it('selectTraceById returns trace or null', () => {
      agentTraceStore.handleWsEvent(wire('debug_system_prompt', {
        workflow_id: 'wf-1', step_id: 's-1', agent_execution_id: 'ae-1', agent_name: 'X', content: 'p',
      }))

      const state = agentTraceStore.store.getState()
      expect(agentTraceStore.selectTraceById('ae-1')(state)).not.toBeNull()
      expect(agentTraceStore.selectTraceById('nope')(state)).toBeNull()
    })
  })
})
