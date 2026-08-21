import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { workflowLiveStore } from '.'
import { startLiveSync, stopLiveSync, ACTIVE_POLL_MS, IDLE_POLL_MS } from './sync'
import { workflowExecutionStore } from '../workflowExecutionStore'

const { mockHydrate } = vi.hoisted(() => ({
  mockHydrate: vi.fn(() => Promise.resolve()),
}))

vi.mock('./hydrate', () => ({
  hydrateLiveState: mockHydrate,
  hydrateActive: vi.fn(() => Promise.resolve()),
}))

const flush = async () => {
  await vi.advanceTimersByTimeAsync(0)
}

beforeEach(() => {
  vi.useFakeTimers()
  vi.clearAllMocks()
  workflowLiveStore.store.setState({
    isGenerating: false,
    hydratedAt: '2025-01-01T00:00:00Z',
    error: null,
    consecutiveFailures: 0,
  })
  workflowExecutionStore.reset()
  mockHydrate.mockImplementation(() => Promise.resolve())
})

afterEach(() => {
  stopLiveSync()
  vi.useRealTimers()
})

describe('startLiveSync', () => {
  it('hydrates immediately', async () => {
    startLiveSync('wf-1')
    await flush()

    expect(mockHydrate).toHaveBeenCalledWith('wf-1')
  })

  it('polls fast while generating', async () => {
    workflowLiveStore.store.setState({ isGenerating: true })
    startLiveSync('wf-1')
    await flush()
    mockHydrate.mockClear()

    await vi.advanceTimersByTimeAsync(ACTIVE_POLL_MS)

    expect(mockHydrate).toHaveBeenCalledTimes(1)
  })

  it('polls fast while a run is in flight', async () => {
    workflowExecutionStore.store.setState({ isRunning: true })
    startLiveSync('wf-1')
    await flush()
    mockHydrate.mockClear()

    await vi.advanceTimersByTimeAsync(ACTIVE_POLL_MS)

    expect(mockHydrate).toHaveBeenCalledTimes(1)
  })

  it('backs off to the idle cadence when nothing is happening', async () => {
    startLiveSync('wf-1')
    await flush()
    mockHydrate.mockClear()

    await vi.advanceTimersByTimeAsync(ACTIVE_POLL_MS)
    expect(mockHydrate).not.toHaveBeenCalled()

    await vi.advanceTimersByTimeAsync(IDLE_POLL_MS - ACTIVE_POLL_MS)
    expect(mockHydrate).toHaveBeenCalledTimes(1)
  })

  it('does not stack timers when called twice for the same workflow', async () => {
    startLiveSync('wf-1')
    await flush()
    startLiveSync('wf-1')
    await flush()
    mockHydrate.mockClear()

    await vi.advanceTimersByTimeAsync(IDLE_POLL_MS)

    expect(mockHydrate).toHaveBeenCalledTimes(1)
  })

  it('backs off after consecutive failures', async () => {
    workflowLiveStore.store.setState({ isGenerating: true })
    // hydrateLiveState owns the counter; the poller only reads it.
    mockHydrate.mockImplementation(() => {
      const n = workflowLiveStore.store.getState().consecutiveFailures + 1
      workflowLiveStore.store.setState({ error: 'boom', consecutiveFailures: n })
      return Promise.resolve()
    })

    startLiveSync('wf-1')
    await flush()
    mockHydrate.mockClear()

    // One failure recorded — the next wait is longer than the active cadence.
    await vi.advanceTimersByTimeAsync(ACTIVE_POLL_MS)
    expect(mockHydrate).not.toHaveBeenCalled()

    await vi.advanceTimersByTimeAsync(ACTIVE_POLL_MS * 2)
    expect(mockHydrate).toHaveBeenCalledTimes(1)
  })
})

describe('stopLiveSync', () => {
  it('stops further polling', async () => {
    startLiveSync('wf-1')
    await flush()
    mockHydrate.mockClear()

    stopLiveSync()
    await vi.advanceTimersByTimeAsync(IDLE_POLL_MS * 3)

    expect(mockHydrate).not.toHaveBeenCalled()
  })
})

describe('startLiveSync deduplication', () => {
  it('skips the immediate fetch when the caller already hydrated', async () => {
    // The editor page hydrates on mount, then starts the sync — that must not
    // fire a second request for the same workflow.
    workflowLiveStore.store.setState({ workflowId: 'wf-1', hydratedAt: '2025-01-01T00:00:00Z' })

    startLiveSync('wf-1')
    await flush()

    expect(mockHydrate).not.toHaveBeenCalled()
  })

  it('still fetches immediately for a workflow it has no data for', async () => {
    workflowLiveStore.store.setState({ workflowId: null, hydratedAt: null })

    startLiveSync('wf-2')
    await flush()

    expect(mockHydrate).toHaveBeenCalledWith('wf-2')
  })
})
