import { describe, it, expect } from 'vitest'
import { activityMessage } from './activityMessages'
import type { ActivityEvent } from '@/types/activity'

describe('activityMessage', () => {
  // ── Workflow ──────────────────────────────────────────────────────────

  it('workflow:started', () => {
    const event: ActivityEvent = { type: 'workflow:started', workflowId: 'wf-1', totalSteps: 5 }
    expect(activityMessage(event)).toBe('Workflow started (5 steps)')
  })

  it('workflow:started singular', () => {
    const event: ActivityEvent = { type: 'workflow:started', workflowId: 'wf-1', totalSteps: 1 }
    expect(activityMessage(event)).toBe('Workflow started (1 step)')
  })

  it('workflow:step_started', () => {
    const event: ActivityEvent = { type: 'workflow:step_started', workflowId: 'wf-1', stepId: 's-1', stepName: 'Analyze', agentId: null, executionId: null }
    expect(activityMessage(event)).toBe('Step "Analyze" started')
  })

  it('workflow:step_completed with duration', () => {
    const event: ActivityEvent = { type: 'workflow:step_completed', workflowId: 'wf-1', stepId: 's-1', stepName: 'Analyze', agentId: null, output: null, inputTokens: null, outputTokens: null, durationMs: 1200 }
    expect(activityMessage(event)).toBe('Step "Analyze" completed (1200ms)')
  })

  it('workflow:step_completed without duration', () => {
    const event: ActivityEvent = { type: 'workflow:step_completed', workflowId: 'wf-1', stepId: 's-1', stepName: 'Analyze', agentId: null, output: null, inputTokens: null, outputTokens: null, durationMs: null }
    expect(activityMessage(event)).toBe('Step "Analyze" completed')
  })

  it('workflow:step_failed', () => {
    const event: ActivityEvent = { type: 'workflow:step_failed', workflowId: 'wf-1', stepId: 's-1', stepName: 'Analyze', error: 'timeout' }
    expect(activityMessage(event)).toContain('FAILED')
    expect(activityMessage(event)).toContain('timeout')
  })

  it('workflow:step_paused', () => {
    const event: ActivityEvent = { type: 'workflow:step_paused', workflowId: 'wf-1', stepId: 's-1', stepName: 'Review' }
    expect(activityMessage(event)).toBe('Step "Review" paused')
  })

  it('workflow:for_each_progress', () => {
    const event: ActivityEvent = { type: 'workflow:for_each_progress', workflowId: 'wf-1', stepId: 's-1', stepName: 'Process', completed: 3, total: 10 }
    expect(activityMessage(event)).toBe('Step "Process" progress: 3/10')
  })

  it('workflow:completed with duration', () => {
    const event: ActivityEvent = { type: 'workflow:completed', workflowId: 'wf-1', durationMs: 5000 }
    expect(activityMessage(event)).toBe('Workflow completed (5000ms)')
  })

  it('workflow:completed without duration', () => {
    const event: ActivityEvent = { type: 'workflow:completed', workflowId: 'wf-1', durationMs: null }
    expect(activityMessage(event)).toBe('Workflow completed')
  })

  it('workflow:failed', () => {
    const event: ActivityEvent = { type: 'workflow:failed', workflowId: 'wf-1', error: 'out of memory' }
    expect(activityMessage(event)).toContain('FAILED')
    expect(activityMessage(event)).toContain('out of memory')
  })

  it('workflow:resumed', () => {
    const event: ActivityEvent = { type: 'workflow:resumed', workflowId: 'wf-1', stepId: 's-1' }
    expect(activityMessage(event)).toContain('resumed')
  })

  // ── Room ──────────────────────────────────────────────────────────────

  it('room:speaker_start', () => {
    const event: ActivityEvent = { type: 'room:speaker_start', roomSessionId: 'rs-1', agentId: 'a-1', agentName: 'Alice', speakerOrder: 1, turnNumber: 2 }
    expect(activityMessage(event)).toContain('Alice')
    expect(activityMessage(event)).toContain('speaking')
  })

  it('room:speaker_token', () => {
    const event: ActivityEvent = { type: 'room:speaker_token', roomSessionId: 'rs-1', agentId: 'a-1', agentName: 'Alice', content: 'Hello world', speakerOrder: 1, turnNumber: 2 }
    expect(activityMessage(event)).toContain('Alice')
    expect(activityMessage(event)).toContain('Hello world')
  })

  it('room:speaker_end', () => {
    const event: ActivityEvent = { type: 'room:speaker_end', roomSessionId: 'rs-1', agentId: 'a-1', agentName: 'Alice', content: 'Full message', speakerOrder: 1, turnNumber: 2 }
    expect(activityMessage(event)).toContain('Alice')
    expect(activityMessage(event)).toContain('finished')
  })

  it('room:turn_complete', () => {
    const event: ActivityEvent = { type: 'room:turn_complete', roomSessionId: 'rs-1', turnNumber: 3 }
    expect(activityMessage(event)).toBe('Turn 3 complete')
  })

  it('room:session_complete', () => {
    const event: ActivityEvent = { type: 'room:session_complete', roomSessionId: 'rs-1', turnNumber: 5 }
    expect(activityMessage(event)).toContain('complete')
    expect(activityMessage(event)).toContain('5 turns')
  })

  it('room:session_complete singular', () => {
    const event: ActivityEvent = { type: 'room:session_complete', roomSessionId: 'rs-1', turnNumber: 1 }
    expect(activityMessage(event)).toContain('1 turn)')
  })

  // ── Session ───────────────────────────────────────────────────────────

  it('session:created', () => {
    const event: ActivityEvent = { type: 'session:created', sessionId: 'sess-1', title: 'Test', modeId: 'mode-1' }
    expect(activityMessage(event)).toContain('Test')
    expect(activityMessage(event)).toContain('created')
  })

  it('session:updated with title', () => {
    const event: ActivityEvent = { type: 'session:updated', sessionId: 'sess-1', title: 'New Title', modeId: null }
    expect(activityMessage(event)).toContain('New Title')
  })

  it('session:updated without title', () => {
    const event: ActivityEvent = { type: 'session:updated', sessionId: 'sess-1', title: null, modeId: 'mode-2' }
    expect(activityMessage(event)).toBe('Session updated')
  })

  it('session:deleted', () => {
    const event: ActivityEvent = { type: 'session:deleted', sessionId: 'sess-1' }
    expect(activityMessage(event)).toContain('deleted')
  })
})
