import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { AgentsPage } from './AgentsPage'
import type { Agent } from '@/types/agent'

const mockNavigate = vi.hoisted(() => vi.fn())
const mockCreateSession = vi.hoisted(() => vi.fn())
const mockRemoveAgent = vi.hoisted(() => vi.fn())

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom')
  return { ...actual, useNavigate: () => mockNavigate }
})

const mockAgents: Agent[] = [
  {
    id: '1',
    name: 'Alice Agent',
    system_prompt: 'You are a helpful assistant',
    model_provider: 'openai',
    model_id: 'gpt-4',
    model_max_tokens: 4096,
    model_temperature: 0.7,
    status: 'idle',
    output_schema_id: null,
    version: 1,
  },
  {
    id: '2',
    name: 'Bob Agent',
    system_prompt: 'You are a coding assistant',
    model_provider: 'anthropic',
    model_id: 'claude-3',
    model_max_tokens: 8192,
    model_temperature: 0.5,
    status: 'working',
    output_schema_id: null,
    version: 1,
  },
]

let mockStoreAgents = mockAgents
let mockStoreLoading = false
let mockStoreError: string | null = null

vi.mock('@/stores/agentStore', () => ({
  agentStore: {
    store: {
      getState: () => ({}),
      subscribe: () => () => {},
    },
    selectAll: () => mockStoreAgents,
    selectLoading: () => mockStoreLoading,
    selectError: () => mockStoreError,
    fetchAll: vi.fn().mockResolvedValue(undefined),
    remove: mockRemoveAgent,
  },
}))

let mockStoreSessions: unknown[] = []
let mockSessionsLoading = false

vi.mock('@/stores/sessionStore', () => ({
  sessionStore: {
    store: {
      getState: () => ({}),
      subscribe: () => () => {},
    },
    selectAll: () => mockStoreSessions,
    selectLoading: () => mockSessionsLoading,
    selectError: () => null,
    fetchAll: vi.fn().mockResolvedValue(undefined),
  },
}))

vi.mock('@/api', () => ({
  api: {
    sessions: { create: mockCreateSession },
  },
}))

describe('AgentsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStoreAgents = mockAgents
    mockStoreLoading = false
    mockStoreError = null
    mockStoreSessions = []
    mockSessionsLoading = false
  })

  it('renders page header', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Agents')).toBeInTheDocument()
    expect(screen.getByText('New Workshop')).toBeInTheDocument()
  })

  it('renders agents in table', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Alice Agent')).toBeInTheDocument()
    expect(screen.getByText('Bob Agent')).toBeInTheDocument()
  })

  it('displays all agent columns', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    // Check column headers
    expect(screen.getByText('Name')).toBeInTheDocument()
    expect(screen.getByText('System Prompt')).toBeInTheDocument()
    expect(screen.getByText('Model')).toBeInTheDocument()
    expect(screen.getByText('Temperature')).toBeInTheDocument()
    expect(screen.getByText('Max Tokens')).toBeInTheDocument()
    expect(screen.getByText('Status')).toBeInTheDocument()
    expect(screen.getByText('Actions')).toBeInTheDocument()

    // Check data is displayed
    expect(screen.getByText('openai/gpt-4')).toBeInTheDocument()
    expect(screen.getByText('anthropic/claude-3')).toBeInTheDocument()
    expect(screen.getByText('0.7')).toBeInTheDocument()
    expect(screen.getByText('4,096')).toBeInTheDocument()
  })

  it('shows loading state', () => {
    mockStoreAgents = []
    mockStoreLoading = true

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Loading data...')).toBeInTheDocument()
  })

  it('shows empty state when no agents', () => {
    mockStoreAgents = []

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('No agents yet. Create your first one in the workshop!')).toBeInTheDocument()
  })

  it('shows error message', () => {
    mockStoreAgents = []
    mockStoreError = 'Failed to load agents'

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Failed to load agents')).toBeInTheDocument()
  })

  it('has search functionality', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    const searchInput = screen.getByPlaceholderText('Search agents by name, model, or prompt...')
    expect(searchInput).toBeInTheDocument()
  })

  it('renders draft agents alongside regular agents', () => {
    const agentsWithDraft = [
      ...mockAgents,
      {
        ...mockAgents[0],
        id: '3',
        name: '[Workshop Draft] Test',
      },
    ]

    mockStoreAgents = agentsWithDraft

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('[Workshop Draft] Test')).toBeInTheDocument()
    expect(screen.getByText('Alice Agent')).toBeInTheDocument()
    expect(screen.getByText('Bob Agent')).toBeInTheDocument()
  })

  it('shows action menu buttons for each agent', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    // Should have action menu buttons for each agent
    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    expect(actionButtons).toHaveLength(2)
  })

  it('shows Workshop action in menu', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    // Open first action menu
    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    await user.click(actionButtons[0])

    // Should show Start Workshop action
    expect(screen.getByText('Start Workshop')).toBeInTheDocument()
  })

  it('shows Open Workshop when session exists', async () => {
    const user = userEvent.setup()

    mockStoreSessions = [
      {
        id: 'session-1',
        agent_id: '1',
        mode_id: 'workshop',
        title: 'Workshop',
      },
    ]

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    // Open first action menu
    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    await user.click(actionButtons[0])

    expect(screen.getByText('Open Workshop')).toBeInTheDocument()
  })

  it('shows View Details and Delete actions in menu', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    // Open first action menu
    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    await user.click(actionButtons[0])

    expect(screen.getByText('View Details')).toBeInTheDocument()
    expect(screen.getByText('Delete')).toBeInTheDocument()
  })

  it('navigates to workshop on New Workshop button click', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    const newWorkshopButton = screen.getByText('New Workshop')
    await user.click(newWorkshopButton)

    expect(mockNavigate).toHaveBeenCalledWith('/agents/workshop')
  })

  it('has sortable columns', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    // Name column should have sort label
    const nameHeader = screen.getByText('Name')
    expect(nameHeader.closest('span')).toHaveClass('MuiTableSortLabel-root')
  })

  it('has pagination', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Rows per page:')).toBeInTheDocument()
  })

  it('has column visibility menu', () => {
    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    const columnMenuButton = screen.getByLabelText('Column visibility')
    expect(columnMenuButton).toBeInTheDocument()
  })

  it('opens delete confirmation modal when delete action clicked', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <AgentsPage />
      </MemoryRouter>,
    )

    // Open first action menu
    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    await user.click(actionButtons[0])

    // Click delete action
    const deleteAction = screen.getByText('Delete')
    await user.click(deleteAction)

    // Confirmation modal should open
    expect(screen.getByText('Delete Agent')).toBeInTheDocument()
    expect(screen.getByText(/Are you sure you want to delete "Alice Agent"/i)).toBeInTheDocument()
  })
})
