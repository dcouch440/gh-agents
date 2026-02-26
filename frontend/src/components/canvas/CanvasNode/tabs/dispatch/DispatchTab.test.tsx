import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { DispatchTab } from './DispatchTab'
import type { DispatchEntry } from '@/stores/dispatchStore'

const mockEntry: DispatchEntry = {
  executionId: 'exec-1',
  stepId: 'step-1',
  status: 'running',
  instruction: 'Set up the search agent',
  message: null,
  summary: null,
  error: null,
  startedAt: '2025-01-01T00:00:00Z',
  trace: [
    { type: 'token', content: 'Working on it...', ts: '2025-01-01T00:00:01Z' },
  ],
  tokenBuffer: 'Working on it...',
}

// Track what the selectors return
let mockActiveEntry: DispatchEntry | null = null
let mockCompletedEntry: DispatchEntry | null = null
let mockMessages: unknown[] = []
let mockIsLoading = false
let mockError: string | null = null

vi.mock('@/stores', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/stores')>()
  return {
    ...actual,
    useStore: (_store: unknown, selector: (s: unknown) => unknown) => {
      // The selector is curried — it returns a function that takes state.
      // But useStore calls it directly with the store state.
      // We identify which selector was called by what it returns.
      const result = selector({
        byStep: {
          'step-1': mockCompletedEntry,
        },
      })
      return result
    },
    dispatchStore: {
      store: {},
      selectByStepId: (stepId: string) => (s: { byStep: Record<string, DispatchEntry | null> }) => s.byStep[stepId] ?? null,
      selectActiveForStep: (_stepId: string) => () => mockActiveEntry,
      hydrateFromApi: vi.fn(),
    },
    dispatchSessionStore: {
      store: {},
      selectMessages: (_stepId: string) => () => mockMessages,
      selectLoading: (_stepId: string) => () => mockIsLoading,
      selectError: (_stepId: string) => () => mockError,
      loadSession: vi.fn(),
      appendDispatchResult: vi.fn(),
    },
  }
})

vi.mock('./DispatchTraceView', () => ({
  DispatchTraceView: ({ entry }: { entry: DispatchEntry }) => (
    <div data-testid="trace-view">{entry.status}</div>
  ),
}))

describe('DispatchTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockActiveEntry = null
    mockCompletedEntry = null
    mockMessages = []
    mockIsLoading = false
    mockError = null
  })

  it('renders empty state when no dispatch entry exists', () => {
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByText('No dispatch activity yet.')).toBeInTheDocument()
  })

  it('renders instruction header when instruction is present', () => {
    mockActiveEntry = mockEntry
    mockCompletedEntry = mockEntry
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByText('Set up the search agent')).toBeInTheDocument()
  })

  it('renders DispatchTraceView when entry exists', () => {
    mockActiveEntry = mockEntry
    mockCompletedEntry = mockEntry
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByTestId('trace-view')).toHaveTextContent('running')
  })

  it('renders completed entry', () => {
    const completedEntry = { ...mockEntry, status: 'completed' as const }
    mockActiveEntry = completedEntry
    mockCompletedEntry = completedEntry
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByTestId('trace-view')).toHaveTextContent('completed')
  })
})
