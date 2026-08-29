import { describe, it, expect, beforeEach } from 'vitest'
import { dispatchStore } from './dispatchStore'
import { SESSION_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import type { DispatchTraceResponse } from '@/types/dispatch'

const makeTrace = (overrides: Partial<DispatchTraceResponse> = {}): DispatchTraceResponse => ({
  execution_id: 'exec-1',
  step_id: 'step-1',
  workflow_id: 'wf-1',
  status: 'completed',
  instruction: 'Configure Research node',
  trace: [{ type: 'token', content: 'hello', ts: '2025-01-01T00:00:01Z' }],
  result: 'done',
  ...overrides,
})

const makeMsg = (event: string, data: Record<string, unknown>): WsWireMessage => ({
  topic: 'session',
  event,
  ts: '2025-01-01T00:00:00Z',
  run_id: null,
  user_id: null,
  seq: null,
  data,
})

beforeEach(() => {
  dispatchStore.store.setState({ byStep: {} })
})

describe('hydrateFromApi', () => {
  it('creates an entry from a REST trace', () => {
    dispatchStore.hydrateFromApi(makeTrace())

    const entry = dispatchStore.store.getState().byStep['step-1']
    expect(entry?.status).toBe('completed')
    expect(entry?.tokenBuffer).toBe('hello')
  })

  it('updates a running entry instead of bailing out', () => {
    // The old guard refused to touch a 'running' entry, so a dispatch that
    // finished while the socket was down stayed spinning forever.
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_STARTED, {
      step_id: 'step-1',
      execution_id: 'exec-1',
      instruction: 'Configure Research node',
    }))
    expect(dispatchStore.store.getState().byStep['step-1']?.status).toBe('running')

    dispatchStore.hydrateFromApi(makeTrace({ status: 'completed' }))

    expect(dispatchStore.store.getState().byStep['step-1']?.status).toBe('completed')
  })

  it('reads a failed dispatch result as the error, not a summary', () => {
    // mark_failed stores the failure text in the task's `result`, so filing it
    // as a summary lost the reason a design failed on every refresh.
    dispatchStore.hydrateFromApi(makeTrace({
      status: 'failed',
      result: 'System node agent timed out after 120s',
    }))

    const entry = dispatchStore.store.getState().byStep['step-1']
    expect(entry?.error).toBe('System node agent timed out after 120s')
    expect(entry?.summary).toBeNull()
  })

  it('keeps an error delivered over the socket when REST has no result', () => {
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_FAILED, {
      step_id: 'step-1',
      error: 'container create failed',
    }))

    dispatchStore.hydrateFromApi(makeTrace({ status: 'failed', result: null }))

    expect(dispatchStore.store.getState().byStep['step-1']?.error).toBe('container create failed')
  })

  it('keeps the longer local trace when WebSocket is ahead of REST', () => {
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_STARTED, {
      step_id: 'step-1', execution_id: 'exec-1', instruction: 'i',
    }))
    for (const content of ['a', 'b', 'c']) {
      dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_STREAM_TOKEN, { step_id: 'step-1', content }))
    }

    dispatchStore.hydrateFromApi(makeTrace({ status: 'running' }))

    const entry = dispatchStore.store.getState().byStep['step-1']
    expect(entry?.trace).toHaveLength(3)
    expect(entry?.tokenBuffer).toBe('abc')
  })

  it('takes the incoming trace when REST is ahead', () => {
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_STARTED, {
      step_id: 'step-1', execution_id: 'exec-1', instruction: 'i',
    }))

    dispatchStore.hydrateFromApi(makeTrace({
      trace: [
        { type: 'token', content: 'x', ts: '2025-01-01T00:00:01Z' },
        { type: 'token', content: 'y', ts: '2025-01-01T00:00:02Z' },
      ],
    }))

    expect(dispatchStore.store.getState().byStep['step-1']?.tokenBuffer).toBe('xy')
  })

  it('lets a live running status survive a pre-start snapshot', () => {
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_STARTED, {
      step_id: 'step-1', execution_id: 'exec-1', instruction: 'i',
    }))

    dispatchStore.hydrateFromApi(makeTrace({ status: 'pending', trace: [] }))

    expect(dispatchStore.store.getState().byStep['step-1']?.status).toBe('running')
  })

  it('preserves the local phase marker', () => {
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_STARTED, {
      step_id: 'step-1', execution_id: 'exec-1', instruction: 'i',
    }))
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_PROGRESS, {
      step_id: 'step-1', message: 'designing agents',
    }))

    dispatchStore.hydrateFromApi(makeTrace())

    expect(dispatchStore.store.getState().byStep['step-1']?.message).toBe('designing agents')
  })
})

describe('upsert on early stream events', () => {
  it('creates an entry when a token arrives before dispatch_started', () => {
    // These used to be dropped, which is how the panel ended up blank after a
    // refresh landed mid-dispatch.
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_STREAM_TOKEN, {
      step_id: 'step-1',
      content: 'partial',
    }))

    const entry = dispatchStore.store.getState().byStep['step-1']
    expect(entry?.tokenBuffer).toBe('partial')
    expect(entry?.status).toBe('running')
  })

  it('creates an entry when a progress marker arrives first', () => {
    dispatchStore.handleWsEvent(makeMsg(SESSION_EVENT.DISPATCH_PROGRESS, {
      step_id: 'step-1',
      message: 'designing agents',
    }))

    expect(dispatchStore.store.getState().byStep['step-1']?.message).toBe('designing agents')
  })
})

describe('pruneToSteps', () => {
  it('drops entries for steps that are no longer reported', () => {
    dispatchStore.hydrateFromApi(makeTrace({ step_id: 'keep' }))
    dispatchStore.hydrateFromApi(makeTrace({ step_id: 'drop', execution_id: 'exec-2' }))

    dispatchStore.pruneToSteps(['keep'])

    const byStep = dispatchStore.store.getState().byStep
    expect(byStep['keep']).toBeDefined()
    expect(byStep['drop']).toBeUndefined()
  })

  it('leaves the store untouched when everything is still present', () => {
    dispatchStore.hydrateFromApi(makeTrace({ step_id: 'keep' }))
    const before = dispatchStore.store.getState().byStep

    dispatchStore.pruneToSteps(['keep'])

    expect(dispatchStore.store.getState().byStep).toBe(before)
  })
})
