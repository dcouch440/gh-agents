import { describe, it, expect, beforeEach, vi } from 'vitest'
import { activityStore } from './activityStore'
import type { WsWireMessage } from '@/types/ws'

const makeMsg = (topic: string, event: string, data: Record<string, unknown>, overrides?: Partial<WsWireMessage>): WsWireMessage => ({
  topic: topic as WsWireMessage['topic'],
  event,
  ts: '2025-01-01T00:00:00Z',
  run_id: 'run-1',
  user_id: 'user-1',
  data,
  ...overrides,
})

describe('activityStore', () => {
  beforeEach(() => {
    activityStore.reset()
  })

  // ── handleWsEvent ──────────────────────────────────────────────────────

  describe('handleWsEvent', () => {
    it('appends a parsed event as an ActivityEntry', () => {
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 3 }))

      const entries = activityStore.selectAll(activityStore.store.getState())
      expect(entries).toHaveLength(1)
      expect(entries[0].event).toEqual({ type: 'workflow:started', workflowId: 'wf-1', totalSteps: 3 })
    })

    it('assigns monotonic seq numbers', () => {
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 3 }))
      activityStore.handleWsEvent(makeMsg('workflow', 'completed', { workflow_id: 'wf-1', duration_ms: 1000 }))

      const entries = activityStore.selectAll(activityStore.store.getState())
      expect(entries[0].seq).toBeLessThan(entries[1].seq)
    })

    it('preserves wire envelope metadata', () => {
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 1 }, {
        ts: '2025-06-15T12:00:00Z',
        run_id: 'run-abc',
        user_id: 'user-xyz',
      }))

      const entry = activityStore.selectAll(activityStore.store.getState())[0]
      expect(entry.ts).toBe('2025-06-15T12:00:00Z')
      expect(entry.runId).toBe('run-abc')
      expect(entry.userId).toBe('user-xyz')
    })

    it('records receivedAt as a timestamp', () => {
      const before = Date.now()
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 1 }))
      const after = Date.now()

      const entry = activityStore.selectAll(activityStore.store.getState())[0]
      expect(entry.receivedAt).toBeGreaterThanOrEqual(before)
      expect(entry.receivedAt).toBeLessThanOrEqual(after)
    })

    it('silently drops unknown events', () => {
      activityStore.handleWsEvent(makeMsg('workflow', 'nonexistent', {}))
      expect(activityStore.selectAll(activityStore.store.getState())).toHaveLength(0)
    })

    it('silently drops unknown topics', () => {
      activityStore.handleWsEvent(makeMsg('unknown_topic', 'started', {}))
      expect(activityStore.selectAll(activityStore.store.getState())).toHaveLength(0)
    })

    it('handles null run_id and user_id', () => {
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 1 }, {
        run_id: null,
        user_id: null,
      }))

      const entry = activityStore.selectAll(activityStore.store.getState())[0]
      expect(entry.runId).toBeNull()
      expect(entry.userId).toBeNull()
    })
  })

  // ── Rolling window ─────────────────────────────────────────────────────

  describe('rolling window', () => {
    it('trims oldest entries when exceeding maxSize', () => {
      // Temporarily set a small maxSize for testing
      activityStore.store.setState({ maxSize: 3 })

      for (let i = 0; i < 5; i++) {
        activityStore.handleWsEvent(makeMsg('session', 'created', {
          session_id: `sess-${i}`, title: `Session ${i}`, mode_id: 'mode-1',
        }))
      }

      const entries = activityStore.selectAll(activityStore.store.getState())
      expect(entries).toHaveLength(3)
      // Should keep the 3 most recent
      expect(entries[0].event).toMatchObject({ sessionId: 'sess-2' })
      expect(entries[1].event).toMatchObject({ sessionId: 'sess-3' })
      expect(entries[2].event).toMatchObject({ sessionId: 'sess-4' })
    })
  })

  // ── Selectors ──────────────────────────────────────────────────────────

  describe('selectors', () => {
    const seedEntries = (): void => {
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 3 }, { run_id: 'run-1' }))
      activityStore.handleWsEvent(makeMsg('workflow', 'step_failed', { workflow_id: 'wf-1', step_id: 's-1', step_name: 'Analyze', error: 'timeout' }, { run_id: 'run-1' }))
      activityStore.handleWsEvent(makeMsg('room', 'speaker_start', { room_session_id: 'rs-1', agent_id: 'a-1', agent_name: 'Alice', speaker_order: 1, turn_number: 1 }, { run_id: 'run-2' }))
      activityStore.handleWsEvent(makeMsg('session', 'created', { session_id: 'sess-1', title: 'Test', mode_id: 'mode-1' }, { run_id: null }))
      activityStore.handleWsEvent(makeMsg('workflow', 'failed', { workflow_id: 'wf-1', error: 'fatal' }, { run_id: 'run-1' }))
    }

    it('selectAll returns all entries', () => {
      seedEntries()
      expect(activityStore.selectAll(activityStore.store.getState())).toHaveLength(5)
    })

    it('selectByRunId filters by run', () => {
      seedEntries()
      const run1 = activityStore.selectByRunId('run-1')(activityStore.store.getState())
      expect(run1).toHaveLength(3)
      run1.forEach((e) => expect(e.runId).toBe('run-1'))
    })

    it('selectByTopic filters by topic prefix', () => {
      seedEntries()
      const workflow = activityStore.selectByTopic('workflow')(activityStore.store.getState())
      expect(workflow).toHaveLength(3)
      workflow.forEach((e) => expect(e.event.type).toMatch(/^workflow:/))

      const room = activityStore.selectByTopic('room')(activityStore.store.getState())
      expect(room).toHaveLength(1)

      const session = activityStore.selectByTopic('session')(activityStore.store.getState())
      expect(session).toHaveLength(1)
    })

    it('selectErrors returns only error events', () => {
      seedEntries()
      const errs = activityStore.selectErrors(activityStore.store.getState())
      expect(errs).toHaveLength(2)
      expect(errs[0].event.type).toBe('workflow:step_failed')
      expect(errs[1].event.type).toBe('workflow:failed')
    })

    it('selectLatest returns last n entries', () => {
      seedEntries()
      const latest = activityStore.selectLatest(2)(activityStore.store.getState())
      expect(latest).toHaveLength(2)
      expect(latest[0].event.type).toBe('session:created')
      expect(latest[1].event.type).toBe('workflow:failed')
    })

    it('selectCount returns entry count', () => {
      seedEntries()
      expect(activityStore.selectCount(activityStore.store.getState())).toBe(5)
    })
  })

  // ── dump ───────────────────────────────────────────────────────────────

  describe('dump', () => {
    it('calls console.table with formatted rows', () => {
      const spy = vi.spyOn(console, 'table').mockImplementation(() => {})

      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 3 }))
      activityStore.dump()

      expect(spy).toHaveBeenCalledOnce()
      const rows = spy.mock.calls[0][0] as Array<Record<string, unknown>>
      expect(rows).toHaveLength(1)
      expect(rows[0]).toHaveProperty('seq')
      expect(rows[0]).toHaveProperty('type', 'workflow:started')
      expect(rows[0]).toHaveProperty('message')
      expect(rows[0]).toHaveProperty('ts')
      expect(rows[0]).toHaveProperty('runId')

      spy.mockRestore()
    })
  })

  // ── reset ──────────────────────────────────────────────────────────────

  describe('reset', () => {
    it('clears all entries and resets seq counter', () => {
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 1 }))
      expect(activityStore.selectCount(activityStore.store.getState())).toBe(1)

      activityStore.reset()
      expect(activityStore.selectCount(activityStore.store.getState())).toBe(0)

      // Seq counter resets — next entry starts at 0
      activityStore.handleWsEvent(makeMsg('workflow', 'started', { workflow_id: 'wf-1', total_steps: 1 }))
      const entries = activityStore.selectAll(activityStore.store.getState())
      expect(entries[0].seq).toBe(0)
    })
  })
})
