import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RouterModesTab } from './RouterModesTab'
import { mockRouterMode, mockToolRouter } from '@/test/fixtures'

const { mockDeleteMode, mockFetchAll, mockFetchModes, mockCreate, mockFetchModeTools, mockSetModeTools, _mockState } = vi.hoisted(() => ({
  mockDeleteMode: vi.fn(),
  mockFetchAll: vi.fn(),
  mockFetchModes: vi.fn(),
  mockCreate: vi.fn(),
  mockFetchModeTools: vi.fn().mockResolvedValue([]),
  mockSetModeTools: vi.fn().mockResolvedValue(undefined),
  _mockState: { routers: [] as unknown[], modes: [] as unknown[] },
}))

vi.mock('@/stores/toolRouterStore', () => ({
  toolRouterStore: {
    store: {
      getState: () => ({}),
      subscribe: () => () => {},
    },
    selectAll: () => _mockState.routers,
    selectLoading: () => false,
    selectError: () => null,
    selectModes: () => () => _mockState.modes,
    fetchAll: mockFetchAll,
    create: mockCreate,
    fetchModes: mockFetchModes,
    deleteMode: mockDeleteMode,
    fetchModeTools: mockFetchModeTools,
    setModeTools: mockSetModeTools,
  },
}))

vi.mock('@/stores/toolStore', () => {
  const emptyArray: never[] = []
  const state = { items: { byId: new Map(), _array: emptyArray, _version: 0 }, loading: false, error: null }
  return {
    toolStore: {
      store: { getState: () => state, subscribe: () => () => {} },
      selectAll: () => emptyArray,
      selectLoading: () => false,
      fetchAll: vi.fn().mockResolvedValue(undefined),
    },
  }
})

vi.mock('./ModeFormDialog', () => ({
  ModeFormDialog: ({ open, mode }: { open: boolean; mode: typeof mockRouterMode | null }) => (
    <div data-testid="mode-form-dialog">{open ? `Dialog: ${mode ? 'Edit' : 'Create'}` : null}</div>
  ),
}))

vi.mock('./ModeToolSelector', () => ({
  ModeToolSelector: ({ open }: { open: boolean }) => <div data-testid="mode-tool-selector">{open ? 'Tool Selector' : null}</div>,
}))

describe('RouterModesTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    _mockState.routers = [mockToolRouter]
    _mockState.modes = [mockRouterMode]
    mockFetchAll.mockResolvedValue(undefined)
    mockFetchModes.mockResolvedValue([mockRouterMode])
    mockCreate.mockResolvedValue(mockToolRouter)
    vi.spyOn(window, 'confirm').mockReturnValue(true)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('rendering', () => {
    it('shows empty state when modes is empty', async () => {
      _mockState.modes = []
      render(<RouterModesTab />)

      expect(await screen.findByText(/no router modes configured/i)).toBeInTheDocument()
    })

    it('shows DataTable when modes has items', async () => {
      render(<RouterModesTab />)

      expect(await screen.findByText(mockRouterMode.mode_key)).toBeInTheDocument()
      expect(screen.getByText(mockRouterMode.display_name)).toBeInTheDocument()
    })
  })

  describe('create mode', () => {
    it('opens ModeFormDialog with mode=null when Create Mode is clicked', async () => {
      const user = userEvent.setup()
      render(<RouterModesTab />)

      const createButton = await screen.findByRole('button', { name: /create mode/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(screen.getByTestId('mode-form-dialog')).toHaveTextContent('Dialog: Create')
      })
    })
  })

  describe('edit mode', () => {
    it('opens ModeFormDialog with selectedMode when Edit is clicked', async () => {
      const user = userEvent.setup()
      render(<RouterModesTab />)

      const editButton = await screen.findByLabelText(/edit mode/i)
      await user.click(editButton)

      await waitFor(() => {
        expect(screen.getByTestId('mode-form-dialog')).toHaveTextContent('Dialog: Edit')
      })
    })
  })

  describe('delete mode', () => {
    it('shows confirm dialog and deletes mode on confirm', async () => {
      const user = userEvent.setup()
      mockDeleteMode.mockResolvedValue(undefined)
      render(<RouterModesTab />)

      const deleteButton = await screen.findByLabelText(/delete mode/i)
      await user.click(deleteButton)

      await waitFor(() => {
        expect(window.confirm).toHaveBeenCalled()
        expect(mockDeleteMode).toHaveBeenCalledWith(mockRouterMode.id)
      })
    })

    it('does not delete when user cancels confirm', async () => {
      const user = userEvent.setup()
      vi.spyOn(window, 'confirm').mockReturnValue(false)
      render(<RouterModesTab />)

      const deleteButton = await screen.findByLabelText(/delete mode/i)
      await user.click(deleteButton)

      expect(window.confirm).toHaveBeenCalled()
      expect(mockDeleteMode).not.toHaveBeenCalled()
    })

    it('shows deleteError on failure', async () => {
      const user = userEvent.setup()
      mockDeleteMode.mockRejectedValue(new Error('Delete failed'))
      render(<RouterModesTab />)

      const deleteButton = await screen.findByLabelText(/delete mode/i)
      await user.click(deleteButton)

      await waitFor(() => {
        expect(screen.getByText('Delete failed')).toBeInTheDocument()
      })
    })
  })

  describe('manage tools', () => {
    it('opens ModeToolSelector when Tools icon is clicked', async () => {
      const user = userEvent.setup()
      render(<RouterModesTab />)

      const toolsButton = await screen.findByLabelText(/manage tools/i)
      await user.click(toolsButton)

      await waitFor(() => {
        expect(screen.getByTestId('mode-tool-selector')).toHaveTextContent('Tool Selector')
      })
    })
  })

  describe('DataTable columns', () => {
    it('renders mode_key as code', async () => {
      render(<RouterModesTab />)

      const modeKeyElement = await screen.findByText(mockRouterMode.mode_key)
      expect(modeKeyElement).toHaveStyle({ fontFamily: 'monospace' })
    })

    it('renders display_name with bold font', async () => {
      render(<RouterModesTab />)

      const displayNameElement = await screen.findByText(mockRouterMode.display_name)
      expect(displayNameElement).toHaveStyle({ fontWeight: 500 })
    })

    it('renders settings chips', async () => {
      render(<RouterModesTab />)

      if (mockRouterMode.append_to_agent_tools) {
        expect(await screen.findByText('Append Tools')).toBeInTheDocument()
      }
      expect(await screen.findByText(`T: ${mockRouterMode.temperature}`)).toBeInTheDocument()
    })

    it('renders action buttons', async () => {
      render(<RouterModesTab />)

      expect(await screen.findByLabelText(/edit mode/i)).toBeInTheDocument()
      expect(screen.getByLabelText(/manage tools/i)).toBeInTheDocument()
      expect(screen.getByLabelText(/delete mode/i)).toBeInTheDocument()
    })
  })
})
