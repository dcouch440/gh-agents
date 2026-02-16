import { describe, it, expect } from 'vitest'
import { parseWsEvent } from './parseWsEvent'
import type { WsWireMessage } from '@/types/ws'

const makeMsg = (topic: string, event: string, data: Record<string, unknown>): WsWireMessage => ({
  topic: topic as WsWireMessage['topic'],
  event,
  ts: '2025-01-01T00:00:00Z',
  run_id: 'run-1',
  user_id: 'user-1',
  data,
})

describe('parseWsEvent', () => {
  // ── Workflow events ──────────────────────────────────────────────────────

  describe('workflow topic', () => {
    it('parses started', () => {
      const result = parseWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 5 }))
      expect(result).toEqual({ type: 'workflow:started', workflowId: 'wf-1', totalSteps: 5 })
    })

    it('parses step_started', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'step_started', {
          workflow_id: 'wf-1',
          step_id: 's-1',
          step_name: 'Analyze',
          agent_id: 'a-1',
          execution_id: 'e-1',
        }),
      )
      expect(result).toEqual({
        type: 'workflow:step_started',
        workflowId: 'wf-1',
        stepId: 's-1',
        stepName: 'Analyze',
        agentId: 'a-1',
        executionId: 'e-1',
      })
    })

    it('parses step_started with null optionals', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'step_started', {
          workflow_id: 'wf-1',
          step_id: 's-1',
          step_name: 'Analyze',
          agent_id: null,
          execution_id: null,
        }),
      )
      expect(result).toMatchObject({ agentId: null, executionId: null })
    })

    it('parses step_completed', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'step_completed', {
          workflow_id: 'wf-1',
          step_id: 's-1',
          step_name: 'Analyze',
          agent_id: 'a-1',
          output: 'done',
          input_tokens: 100,
          output_tokens: 50,
          duration_ms: 1200,
        }),
      )
      expect(result).toEqual({
        type: 'workflow:step_completed',
        workflowId: 'wf-1',
        stepId: 's-1',
        stepName: 'Analyze',
        agentId: 'a-1',
        output: 'done',
        inputTokens: 100,
        outputTokens: 50,
        durationMs: 1200,
      })
    })

    it('parses step_completed with null optionals', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'step_completed', {
          workflow_id: 'wf-1',
          step_id: 's-1',
          step_name: 'Analyze',
        }),
      )
      expect(result).toMatchObject({ output: null, inputTokens: null, outputTokens: null, durationMs: null })
    })

    it('parses step_failed', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'step_failed', {
          workflow_id: 'wf-1',
          step_id: 's-1',
          step_name: 'Analyze',
          error: 'timeout',
        }),
      )
      expect(result).toEqual({
        type: 'workflow:step_failed',
        workflowId: 'wf-1',
        stepId: 's-1',
        stepName: 'Analyze',
        error: 'timeout',
      })
    })

    it('parses step_paused', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'step_paused', {
          workflow_id: 'wf-1',
          step_id: 's-1',
          step_name: 'Review',
        }),
      )
      expect(result).toEqual({
        type: 'workflow:step_paused',
        workflowId: 'wf-1',
        stepId: 's-1',
        stepName: 'Review',
      })
    })

    it('parses for_each_progress', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'for_each_progress', {
          workflow_id: 'wf-1',
          step_id: 's-1',
          step_name: 'Process',
          completed: 3,
          total: 10,
        }),
      )
      expect(result).toEqual({
        type: 'workflow:for_each_progress',
        workflowId: 'wf-1',
        stepId: 's-1',
        stepName: 'Process',
        completed: 3,
        total: 10,
      })
    })

    it('parses completed', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'completed', {
          workflow_id: 'wf-1',
          duration_ms: 5000,
        }),
      )
      expect(result).toEqual({ type: 'workflow:completed', workflowId: 'wf-1', durationMs: 5000 })
    })

    it('parses completed with null duration', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'completed', {
          workflow_id: 'wf-1',
          duration_ms: null,
        }),
      )
      expect(result).toMatchObject({ durationMs: null })
    })

    it('parses failed', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'failed', {
          workflow_id: 'wf-1',
          error: 'out of memory',
        }),
      )
      expect(result).toEqual({ type: 'workflow:failed', workflowId: 'wf-1', error: 'out of memory' })
    })

    it('parses resumed', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'resumed', {
          workflow_id: 'wf-1',
          step_id: 's-1',
        }),
      )
      expect(result).toEqual({ type: 'workflow:resumed', workflowId: 'wf-1', stepId: 's-1' })
    })

    it('parses sub_workflow_started', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'sub_workflow_started', {
          workflow_id: 'wf-1',
          parent_step_id: 'ps-1',
          child_execution_id: 'ce-1',
          total_steps: 3,
        }),
      )
      expect(result).toEqual({
        type: 'workflow:sub_workflow_started',
        workflowId: 'wf-1',
        parentStepId: 'ps-1',
        childExecutionId: 'ce-1',
        totalSteps: 3,
      })
    })

    it('parses sub_workflow_completed', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'sub_workflow_completed', {
          workflow_id: 'wf-1',
          parent_step_id: 'ps-1',
          child_execution_id: 'ce-1',
          status: 'completed',
        }),
      )
      expect(result).toEqual({
        type: 'workflow:sub_workflow_completed',
        workflowId: 'wf-1',
        parentStepId: 'ps-1',
        childExecutionId: 'ce-1',
        status: 'completed',
      })
    })

    it('parses sub_workflow_step_progress', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'sub_workflow_step_progress', {
          workflow_id: 'wf-1',
          parent_step_id: 'ps-1',
          child_execution_id: 'ce-1',
          child_step_id: 'cs-1',
          child_step_name: 'Designer',
          status: 'completed',
          input_tokens: 100,
          output_tokens: 50,
          duration_ms: 2000,
          error: null,
        }),
      )
      expect(result).toEqual({
        type: 'workflow:sub_workflow_step_progress',
        workflowId: 'wf-1',
        parentStepId: 'ps-1',
        childExecutionId: 'ce-1',
        childStepId: 'cs-1',
        childStepName: 'Designer',
        status: 'completed',
        inputTokens: 100,
        outputTokens: 50,
        durationMs: 2000,
        error: null,
      })
    })

    it('parses sub_workflow_step_progress with null optionals', () => {
      const result = parseWsEvent(
        makeMsg('workflow', 'sub_workflow_step_progress', {
          workflow_id: 'wf-1',
          parent_step_id: 'ps-1',
          child_execution_id: 'ce-1',
          child_step_id: 'cs-1',
          child_step_name: 'Agent',
          status: 'started',
        }),
      )
      expect(result).toMatchObject({ inputTokens: null, outputTokens: null, durationMs: null, error: null })
    })

    it('returns null for unknown workflow event', () => {
      expect(parseWsEvent(makeMsg('workflow', 'unknown_event', {}))).toBeNull()
    })
  })

  // ── Room events ────────────────────────────────────────────────────────

  describe('room topic', () => {
    it('parses speaker_start', () => {
      const result = parseWsEvent(
        makeMsg('room', 'speaker_start', {
          room_session_id: 'rs-1',
          agent_id: 'a-1',
          agent_name: 'Alice',
          speaker_order: 1,
          turn_number: 2,
        }),
      )
      expect(result).toEqual({
        type: 'room:speaker_start',
        roomSessionId: 'rs-1',
        agentId: 'a-1',
        agentName: 'Alice',
        speakerOrder: 1,
        turnNumber: 2,
      })
    })

    it('parses speaker_token', () => {
      const result = parseWsEvent(
        makeMsg('room', 'speaker_token', {
          room_session_id: 'rs-1',
          agent_id: 'a-1',
          agent_name: 'Alice',
          content: 'Hello',
          speaker_order: 1,
          turn_number: 2,
        }),
      )
      expect(result).toEqual({
        type: 'room:speaker_token',
        roomSessionId: 'rs-1',
        agentId: 'a-1',
        agentName: 'Alice',
        content: 'Hello',
        speakerOrder: 1,
        turnNumber: 2,
      })
    })

    it('parses speaker_end', () => {
      const result = parseWsEvent(
        makeMsg('room', 'speaker_end', {
          room_session_id: 'rs-1',
          agent_id: 'a-1',
          agent_name: 'Alice',
          content: 'Full message',
          speaker_order: 1,
          turn_number: 2,
        }),
      )
      expect(result).toEqual({
        type: 'room:speaker_end',
        roomSessionId: 'rs-1',
        agentId: 'a-1',
        agentName: 'Alice',
        content: 'Full message',
        speakerOrder: 1,
        turnNumber: 2,
      })
    })

    it('parses turn_complete', () => {
      const result = parseWsEvent(
        makeMsg('room', 'turn_complete', {
          room_session_id: 'rs-1',
          turn_number: 3,
        }),
      )
      expect(result).toEqual({ type: 'room:turn_complete', roomSessionId: 'rs-1', turnNumber: 3 })
    })

    it('parses session_complete', () => {
      const result = parseWsEvent(
        makeMsg('room', 'session_complete', {
          room_session_id: 'rs-1',
          turn_number: 5,
        }),
      )
      expect(result).toEqual({ type: 'room:session_complete', roomSessionId: 'rs-1', turnNumber: 5 })
    })

    it('returns null for unknown room event', () => {
      expect(parseWsEvent(makeMsg('room', 'unknown_event', {}))).toBeNull()
    })
  })

  // ── Session events ─────────────────────────────────────────────────────

  describe('session topic', () => {
    it('parses created', () => {
      const result = parseWsEvent(
        makeMsg('session', 'created', {
          session_id: 'sess-1',
          title: 'Test Session',
          mode_id: 'mode-1',
        }),
      )
      expect(result).toEqual({
        type: 'session:created',
        sessionId: 'sess-1',
        title: 'Test Session',
        modeId: 'mode-1',
      })
    })

    it('parses updated', () => {
      const result = parseWsEvent(
        makeMsg('session', 'updated', {
          session_id: 'sess-1',
          title: 'New Title',
          mode_id: null,
        }),
      )
      expect(result).toEqual({
        type: 'session:updated',
        sessionId: 'sess-1',
        title: 'New Title',
        modeId: null,
      })
    })

    it('parses deleted', () => {
      const result = parseWsEvent(makeMsg('session', 'deleted', { session_id: 'sess-1' }))
      expect(result).toEqual({ type: 'session:deleted', sessionId: 'sess-1' })
    })

    it('returns null for unknown session event', () => {
      expect(parseWsEvent(makeMsg('session', 'unknown_event', {}))).toBeNull()
    })
  })

  // ── Unknown topic ──────────────────────────────────────────────────────

  describe('unknown topic', () => {
    it('returns null for unknown topic', () => {
      expect(parseWsEvent(makeMsg('unknown_topic', 'started', {}))).toBeNull()
    })
  })
})
