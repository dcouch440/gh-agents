import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { WorkflowsPage } from './WorkflowsPage'
import type { Workflow } from '@/types/workflow'

const mockNavigate = vi.hoisted(() => vi.fn())
const mockFetchAll = vi.hoisted(() => vi.fn().mockResolvedValue(undefined))
const mockCreate = vi.hoisted(() => vi.fn())
const mockRemove = vi.hoisted(() => vi.fn())

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom')
  return { ...actual, useNavigate: () => mockNavigate }
})

const mockWorkflows: Workflow[] = [
  {
    id: 'wf-001',
    name: 'Data Pipeline',
    description: 'Processes incoming data',
    created_at: '2025-01-15T10:00:00Z',
    container_enabled: false,
    target_repo_url: null,
    target_branch: null,
    vpn_enabled: false,
  },
  {
    id: 'wf-002',
    name: 'Code Review',
    description: null,
    created_at: '2025-01-20T14:30:00Z',
    container_enabled: true,
    target_repo_url: 'https://github.com/test/repo',
    target_branch: 'main',
    vpn_enabled: false,
  },
]

let mockStoreWorkflows = mockWorkflows
let mockStoreLoading = false
let mockStoreError: string | null = null

vi.mock('@/stores/workflowStore', () => ({
  workflowStore: {
    store: {
      getState: () => ({}),
      subscribe: () => () => {},
    },
    selectAll: () => mockStoreWorkflows,
    selectLoading: () => mockStoreLoading,
    selectError: () => mockStoreError,
    fetchAll: mockFetchAll,
    create: mockCreate,
    remove: mockRemove,
  },
}))

describe('WorkflowsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStoreWorkflows = mockWorkflows
    mockStoreLoading = false
    mockStoreError = null
  })

  // ── Rendering ─────────────────────────────────────────────────────────────

  it('renders page header with title', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Workflows')).toBeInTheDocument()
    expect(screen.getByText('Build and manage AI workflow pipelines.')).toBeInTheDocument()
  })

  it('renders "New Workflow" button', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('New Workflow')).toBeInTheDocument()
  })

  it('shows workflow names in table', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Data Pipeline')).toBeInTheDocument()
    expect(screen.getByText('Code Review')).toBeInTheDocument()
  })

  it('shows workflow descriptions', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Processes incoming data')).toBeInTheDocument()
    expect(screen.getByText('\u2014')).toBeInTheDocument()
  })

  it('renders table column headers', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Name')).toBeInTheDocument()
    expect(screen.getByText('Description')).toBeInTheDocument()
    expect(screen.getByText('Created')).toBeInTheDocument()
    expect(screen.getByText('Actions')).toBeInTheDocument()
  })

  it('shows empty state when no workflows and not loading', () => {
    mockStoreWorkflows = []

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    // EmptyState renders with the action button; table search is absent
    const newWorkflowButtons = screen.getAllByText('New Workflow')
    // One in PageHeader, one in EmptyState action
    expect(newWorkflowButtons.length).toBeGreaterThanOrEqual(2)
    expect(screen.queryByPlaceholderText('Search workflows...')).not.toBeInTheDocument()
  })

  it('shows loading state', () => {
    mockStoreWorkflows = []
    mockStoreLoading = true

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Loading data...')).toBeInTheDocument()
  })

  it('shows error message', () => {
    mockStoreWorkflows = []
    mockStoreError = 'Failed to fetch workflows'

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('Failed to fetch workflows')).toBeInTheDocument()
  })

  it('shows action menu for each workflow', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    expect(actionButtons).toHaveLength(2)
  })

  // ── Data fetching ─────────────────────────────────────────────────────────

  it('calls fetchAll on mount', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(mockFetchAll).toHaveBeenCalledTimes(1)
  })

  // ── Create flow ───────────────────────────────────────────────────────────

  it('shows text field when "New Workflow" is clicked', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    await user.click(screen.getByText('New Workflow'))

    expect(screen.getByPlaceholderText('Workflow name...')).toBeInTheDocument()
    expect(screen.getByText('Create')).toBeInTheDocument()
    expect(screen.getByText('Cancel')).toBeInTheDocument()
  })

  it('hides create form when Cancel is clicked', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    await user.click(screen.getByText('New Workflow'))
    expect(screen.getByPlaceholderText('Workflow name...')).toBeInTheDocument()

    await user.click(screen.getByText('Cancel'))
    expect(screen.queryByPlaceholderText('Workflow name...')).not.toBeInTheDocument()
    expect(screen.getByText('New Workflow')).toBeInTheDocument()
  })

  it('calls create and navigates when typing name and pressing Enter', async () => {
    const user = userEvent.setup()
    const createdWorkflow: Workflow = {
      id: 'wf-new',
      name: 'My Workflow',
      description: null,
      created_at: '2025-02-01T00:00:00Z',
      container_enabled: false,
      target_repo_url: null,
      target_branch: null,
      vpn_enabled: false,
    }
    mockCreate.mockResolvedValue(createdWorkflow)

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    await user.click(screen.getByText('New Workflow'))

    const input = screen.getByPlaceholderText('Workflow name...')
    await user.type(input, 'My Workflow{Enter}')

    expect(mockCreate).toHaveBeenCalledWith({ name: 'My Workflow' })
    expect(mockNavigate).toHaveBeenCalledWith('/workflows/wf-new')
  })

  it('calls create when clicking Create button', async () => {
    const user = userEvent.setup()
    const createdWorkflow: Workflow = {
      id: 'wf-new',
      name: 'Test',
      description: null,
      created_at: '2025-02-01T00:00:00Z',
      container_enabled: false,
      target_repo_url: null,
      target_branch: null,
      vpn_enabled: false,
    }
    mockCreate.mockResolvedValue(createdWorkflow)

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    await user.click(screen.getByText('New Workflow'))

    const input = screen.getByPlaceholderText('Workflow name...')
    await user.type(input, 'Test')
    await user.click(screen.getByText('Create'))

    expect(mockCreate).toHaveBeenCalledWith({ name: 'Test' })
    expect(mockNavigate).toHaveBeenCalledWith('/workflows/wf-new')
  })

  it('does not call create when name is empty', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    await user.click(screen.getByText('New Workflow'))

    const input = screen.getByPlaceholderText('Workflow name...')
    await user.type(input, '{Enter}')

    expect(mockCreate).not.toHaveBeenCalled()
  })

  // ── Delete flow ───────────────────────────────────────────────────────────

  it('opens confirm modal when delete action is clicked', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    await user.click(actionButtons[0])

    const deleteAction = screen.getByText('Delete')
    await user.click(deleteAction)

    await waitFor(() => {
      expect(screen.getByText('Delete Workflow')).toBeInTheDocument()
    })
    expect(screen.getByText(/Are you sure you want to delete/i)).toBeInTheDocument()
  })

  it('shows action menu items including Open Editor, Edit Details, and Delete', async () => {
    const user = userEvent.setup()

    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    const actionButtons = screen.getAllByLabelText(/Actions for/i)
    await user.click(actionButtons[0])

    expect(screen.getByText('Open Editor')).toBeInTheDocument()
    expect(screen.getByText('Edit Details')).toBeInTheDocument()
    expect(screen.getByText('Delete')).toBeInTheDocument()
  })

  // ── Navigation ────────────────────────────────────────────────────────────

  it('has search functionality', () => {
    render(
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>,
    )

    expect(screen.getByPlaceholderText('Search workflows...')).toBeInTheDocument()
  })
})
