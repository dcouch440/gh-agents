import { render, screen } from '@testing-library/react'

const { mockWorkflowId, mockHookReturn } = vi.hoisted(() => ({
  mockWorkflowId: { value: 'wf-001' as string | null },
  mockHookReturn: {
    messages: [] as { id: string; role: string; content: string }[],
    isLoading: false,
    error: null as string | null,
    streaming: false,
    sendMessage: vi.fn(),
    clearHistory: vi.fn(),
  },
}))

vi.mock('@/stores', () => ({
  useStore: () => mockWorkflowId.value,
  workflowStore: {
    store: {},
    selectActiveWorkflowId: () => mockWorkflowId.value,
  },
}))

vi.mock('@/hooks/useAssistantSession', () => ({
  useAssistantSession: () => mockHookReturn,
}))

// Mock ChatPanel to avoid pulling in its full dependency tree
vi.mock('@/components/chat', () => ({
  ChatPanel: ({ emptyMessage }: { emptyMessage?: string }) => (
    <div data-testid="chat-panel">{emptyMessage ?? 'No messages yet'}</div>
  ),
}))

import { AssistantTab } from './AssistantTab'

describe('AssistantTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockWorkflowId.value = 'wf-001'
    mockHookReturn.messages = []
    mockHookReturn.isLoading = false
    mockHookReturn.error = null
    mockHookReturn.streaming = false
  })

  it('renders null when no workflowId', () => {
    mockWorkflowId.value = null
    const { container } = render(<AssistantTab stepId="step-001" />)
    expect(container.innerHTML).toBe('')
  })

  it('renders loading spinner when isLoading', () => {
    mockHookReturn.isLoading = true
    render(<AssistantTab stepId="step-001" />)
    expect(screen.getByRole('progressbar')).toBeInTheDocument()
  })

  it('renders error message on error', () => {
    mockHookReturn.error = 'Connection failed'
    render(<AssistantTab stepId="step-001" />)
    expect(screen.getByText('Connection failed')).toBeInTheDocument()
  })

  it('renders ChatPanel with empty message when ready', () => {
    render(<AssistantTab stepId="step-001" />)
    expect(screen.getByTestId('chat-panel')).toBeInTheDocument()
    expect(screen.getByText('Ask me to help set up documents for this step.')).toBeInTheDocument()
  })

  it('renders AssistantHeader with disabled clear when no messages', () => {
    render(<AssistantTab stepId="step-001" />)
    expect(screen.getByText('Assistant')).toBeInTheDocument()
    const clearButton = screen.getByRole('button')
    expect(clearButton).toBeDisabled()
  })
})
