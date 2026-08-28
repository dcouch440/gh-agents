import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { AgentWorkshopPage } from './AgentWorkshopPage'

const mockNavigate = vi.fn()
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom')
  return { ...actual, useNavigate: () => mockNavigate }
})

const { mockSessionCreate, mockAgentCreate, mockAgentSetContext } = vi.hoisted(() => ({
  mockSessionCreate: vi.fn(),
  mockAgentCreate: vi.fn(),
  mockAgentSetContext: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    agents: { create: mockAgentCreate, update: vi.fn(), get: vi.fn(), getContext: vi.fn(), setContext: mockAgentSetContext },
    sessions: {
      create: mockSessionCreate,
      get: vi.fn(),
      getHistory: vi.fn(),
      clearMessages: vi.fn(),
    },
    outputSchemas: { list: vi.fn().mockResolvedValue({ items: [] }), get: vi.fn(), create: vi.fn(), update: vi.fn(), delete: vi.fn() },
  },
}))

const { mockSend } = vi.hoisted(() => ({ mockSend: vi.fn() }))

vi.mock('@/hooks/useChatMutations', () => ({
  useSendSessionMessage: () => ({
    send: mockSend,
    abort: vi.fn(),
    loading: false,
    streaming: false,
    error: null,
  }),
}))

vi.mock('@/components/primitives/CodeEditor', () => ({
  CodeEditor: ({ value, onChange, placeholder }: { value: string; onChange: (v: string) => void; placeholder?: string }) => (
    <textarea data-testid="code-editor" value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
  ),
}))

vi.mock('@/components/primitives/terminal-renderer', () => ({
  TerminalBlock: ({ content }: { content: string }) => <div data-testid="markdown-preview">{content}</div>,
}))

vi.mock('@/components/DocumentSelector', () => ({
  DocumentSelector: () => null,
}))

const _emptySchemas: never[] = []
vi.mock('@/stores', async () => {
  const actual = await vi.importActual('@/stores')
  const { createStore } = await vi.importActual<{ createStore: <T>(fn: () => T) => unknown }>('zustand/vanilla')
  const emptyStore = createStore(() => ({ items: { byId: {}, ids: [] }, loading: false, error: null, lastFetched: null }))
  return {
    ...actual,
    outputSchemaStore: {
      store: emptyStore,
      selectAll: () => _emptySchemas,
      selectLoading: () => false,
      selectError: () => null,
      fetchAll: vi.fn(),
      fetchIfStale: vi.fn(),
    },
  }
})

const renderPage = () =>
  render(
    <MemoryRouter>
      <AgentWorkshopPage />
    </MemoryRouter>,
  )

describe('AgentWorkshopPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSessionCreate.mockResolvedValue({ id: 'session-001', title: 'Agent Workshop' })
  })

  it('creates a session on mount', async () => {
    renderPage()
    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalledWith(
        expect.objectContaining({
          mode_id: 'workshop',
          title: 'Agent Workshop',
        }),
      )
    })
  })

  it('renders page header and split layout', async () => {
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    expect(screen.getByText('Agent Workshop')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Agent name...')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Save' })).toBeInTheDocument()
  })

  it('renders editor toggle group', async () => {
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    expect(screen.getByRole('button', { name: 'Edit' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Preview' })).toBeInTheDocument()
  })

  it('renders model config fields', async () => {
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    expect(screen.getByLabelText('Model')).toBeInTheDocument()
    expect(screen.getByLabelText('Max Tokens')).toBeInTheDocument()
    expect(screen.getByLabelText('Temperature')).toBeInTheDocument()
  })

  it('disables save when name is empty', async () => {
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled()
  })

  it('enables save when name is filled', async () => {
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    fireEvent.change(screen.getByPlaceholderText('Agent name...'), { target: { value: 'MyAgent' } })
    expect(screen.getByRole('button', { name: 'Save' })).not.toBeDisabled()
  })

  it('calls api.agents.create on save and navigates', async () => {
    mockAgentCreate.mockResolvedValueOnce({ id: 'new-agent' })
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    fireEvent.change(screen.getByPlaceholderText('Agent name...'), { target: { value: 'MyAgent' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(mockAgentCreate).toHaveBeenCalledWith(expect.objectContaining({ name: 'MyAgent' }))
    })

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/agents/workshop/session-001', { replace: true })
    })
  })

  it('displays error on save failure', async () => {
    mockAgentCreate.mockRejectedValueOnce(new Error('Server error'))
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    fireEvent.change(screen.getByPlaceholderText('Agent name...'), { target: { value: 'MyAgent' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(screen.getByText('Server error')).toBeInTheDocument()
    })
  })

  it('displays error on session creation failure', async () => {
    mockSessionCreate.mockRejectedValueOnce(new Error('Session failed'))
    renderPage()

    await waitFor(() => {
      expect(screen.getByText('Session failed')).toBeInTheDocument()
    })
  })

  it('shows chat empty state', async () => {
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    expect(screen.getByText('No messages yet')).toBeInTheDocument()
  })

  it('toggles between edit and preview mode', async () => {
    renderPage()

    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    expect(screen.getByTestId('code-editor')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Preview' }))
    expect(screen.queryByTestId('code-editor')).not.toBeInTheDocument()
  })

  it('sends a chat message after session is ready', async () => {
    mockSend.mockResolvedValue('msg-id')
    renderPage()

    // Wait for session to be created
    await waitFor(() => {
      expect(mockSessionCreate).toHaveBeenCalled()
    })

    // Find the chat textarea (MUI TextField renders a textarea with the placeholder)
    const chatTextarea = screen.getByPlaceholderText('Type a message...')
    fireEvent.change(chatTextarea, { target: { value: 'Hello agent' } })
    fireEvent.keyDown(chatTextarea, { key: 'Enter' })

    await waitFor(() => {
      expect(mockSend).toHaveBeenCalledWith('session-001', { message: 'Hello agent' }, expect.any(Function), expect.any(Function), expect.any(Function))
    })
  })
})
