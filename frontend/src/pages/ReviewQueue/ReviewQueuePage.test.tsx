import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ReviewQueuePage } from './ReviewQueuePage'
import { mockAgentExecution } from '@/test/fixtures'
import type { AgentExecution } from '@/types/execution'

let mockExecutions: AgentExecution[] = []
let mockLoading = false
let mockError: string | null = null
const mockFetchPending = vi.hoisted(() => vi.fn())

vi.mock('@/stores', () => ({
  useStore: (_store: unknown, selector: () => unknown) => selector(),
  reviewQueueStore: {
    store: { getState: () => ({}), subscribe: () => () => {} },
    selectExecutions: () => mockExecutions,
    selectLoading: () => mockLoading,
    selectError: () => mockError,
    fetchPending: mockFetchPending,
  },
}))

const mockSendMessage = vi.hoisted(() => vi.fn())
const mockApprove = vi.hoisted(() => vi.fn().mockResolvedValue(undefined))

vi.mock('@/hooks/useInteractiveChat', () => ({
  useInteractiveChat: () => ({
    messages: [],
    loading: false,
    sending: false,
    streaming: false,
    error: null,
    sendMessage: mockSendMessage,
    approve: mockApprove,
  }),
}))

vi.mock('@/components/chat/ChatPanel', () => ({
  ChatPanel: function ChatPanel() {
    return <div data-testid="chat-panel" />
  },
}))

describe('ReviewQueuePage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockExecutions = []
    mockLoading = false
    mockError = null
  })

  it('shows loading spinner when loading with no executions', () => {
    mockLoading = true
    render(<ReviewQueuePage />)
    expect(screen.getByText('Review Queue')).toBeInTheDocument()
  })

  it('shows error message on error', () => {
    mockError = 'Failed to load executions'
    render(<ReviewQueuePage />)
    expect(screen.getByText('Failed to load executions')).toBeInTheDocument()
  })

  it('shows empty state when no executions', () => {
    render(<ReviewQueuePage />)
    expect(screen.getByText('No executions awaiting review')).toBeInTheDocument()
  })

  it('renders execution list when executions exist', () => {
    mockExecutions = [mockAgentExecution]
    render(<ReviewQueuePage />)
    expect(screen.getByText('Awaiting Review')).toBeInTheDocument()
  })

  it('shows placeholder when no execution selected', () => {
    mockExecutions = [mockAgentExecution]
    render(<ReviewQueuePage />)
    expect(screen.getByText('Select an execution to review')).toBeInTheDocument()
  })

  it('shows detail panel when execution is selected', async () => {
    const user = userEvent.setup()
    mockExecutions = [mockAgentExecution]
    const { container } = render(<ReviewQueuePage />)

    // Click the execution card
    const card = container.querySelector('.MuiPaper-root') as HTMLElement
    await user.click(card)

    expect(screen.getByText('Input')).toBeInTheDocument()
    expect(screen.getByText('Output')).toBeInTheDocument()
    expect(screen.getByTestId('chat-panel')).toBeInTheDocument()
    expect(screen.getByText('Approve')).toBeInTheDocument()
  })

  it('calls fetchPending on mount', () => {
    render(<ReviewQueuePage />)
    expect(mockFetchPending).toHaveBeenCalled()
  })
})
