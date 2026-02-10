import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ModeFormDialog } from './ModeFormDialog'
import { mockRouterMode } from '@/test/fixtures'
import { ApiError } from '@/api'

const { mockCreateMode, mockUpdateMode, mockFetchModeTools, mockSetModeTools } = vi.hoisted(() => ({
  mockCreateMode: vi.fn(),
  mockUpdateMode: vi.fn(),
  mockFetchModeTools: vi.fn(),
  mockSetModeTools: vi.fn(),
}))

vi.mock('@/stores/toolRouterStore', () => ({
  toolRouterStore: {
    store: {
      getState: () => ({}),
      subscribe: () => () => {},
    },
    createMode: mockCreateMode,
    updateMode: mockUpdateMode,
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

describe('ModeFormDialog', () => {
  const mockOnClose = vi.fn()
  const mockOnSave = vi.fn()
  const routerId = 'router-001'

  beforeEach(() => {
    vi.clearAllMocks()
    mockFetchModeTools.mockResolvedValue([])
    mockSetModeTools.mockResolvedValue(undefined)
  })

  describe('rendering', () => {
    it('renders create mode dialog', () => {
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      expect(screen.getByText('Create Router Mode')).toBeInTheDocument()
      expect(screen.getByLabelText(/mode key/i)).not.toBeDisabled()
    })

    it('renders edit mode dialog', () => {
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={mockRouterMode} routerId={routerId} />)

      expect(screen.getByText('Edit Router Mode')).toBeInTheDocument()
      expect(screen.getByLabelText(/mode key/i)).toBeDisabled()
      expect(screen.getByText('Mode key cannot be changed')).toBeInTheDocument()
    })

    it('hydrates form when editing', () => {
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={mockRouterMode} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      const displayNameInput = screen.getByLabelText(/display name/i)
      const descriptionInput = screen.getByLabelText(/description/i)
      const systemPromptInputs = screen.getAllByLabelText(/system prompt/i)
      const systemPromptInput = systemPromptInputs[0] as HTMLTextAreaElement

      expect(modeKeyInput.value).toBe(mockRouterMode.mode_key)
      expect(displayNameInput.value).toBe(mockRouterMode.display_name)
      expect(descriptionInput.value).toBe(mockRouterMode.description)
      expect(systemPromptInput.value).toBe(mockRouterMode.system_prompt)
    })
  })

  describe('form interactions', () => {
    it('allows typing in all enabled fields', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      const displayNameInput = screen.getByLabelText(/display name/i)

      await user.type(modeKeyInput, 'test_mode')
      await user.type(displayNameInput, 'Test Mode')

      expect(modeKeyInput).toHaveValue('test_mode')
      expect(displayNameInput).toHaveValue('Test Mode')
    })

    it('handles checkbox toggles', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const appendPromptCheckbox = screen.getByLabelText(/append to agent system prompt/i)
      const appendToolsCheckbox = screen.getByLabelText(/append to agent tools/i)

      expect(appendPromptCheckbox).not.toBeChecked()
      expect(appendToolsCheckbox).toBeChecked()

      await user.click(appendPromptCheckbox)
      await user.click(appendToolsCheckbox)

      expect(appendPromptCheckbox).toBeChecked()
      expect(appendToolsCheckbox).not.toBeChecked()
    })
  })

  describe('validation', () => {
    it('shows error for empty mode_key', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(screen.getByText('Mode key is required')).toBeInTheDocument()
      })
    })

    it('shows error for invalid mode_key regex', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      await user.type(modeKeyInput, 'Test_Mode')

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(screen.getByText(/must start with a lowercase letter/i)).toBeInTheDocument()
      })
    })

    it('shows error for mode_key > 50 chars', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      await user.type(modeKeyInput, 'a'.repeat(51))

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(screen.getByText(/must be 50 characters or less/i)).toBeInTheDocument()
      })
    })

    it('shows error for empty display_name', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      await user.type(modeKeyInput, 'test_mode')

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(screen.getByText('Display name is required')).toBeInTheDocument()
      })
    })

    it('shows error for temperature out of range', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      const displayNameInput = screen.getByLabelText(/display name/i)
      const temperatureInput = screen.getByLabelText(/temperature/i)

      await user.type(modeKeyInput, 'test_mode')
      await user.type(displayNameInput, 'Test Mode')
      await user.clear(temperatureInput)
      await user.type(temperatureInput, '3.0')

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(screen.getByText(/temperature must be between 0.0 and 2.0/i)).toBeInTheDocument()
      })
    })
  })

  describe('submission - create', () => {
    it('creates mode successfully', async () => {
      const user = userEvent.setup()
      mockCreateMode.mockResolvedValue(mockRouterMode)

      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      const displayNameInput = screen.getByLabelText(/display name/i)

      await user.type(modeKeyInput, 'test_mode')
      await user.type(displayNameInput, 'Test Mode')

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(mockCreateMode).toHaveBeenCalledWith(routerId, {
          mode_key: 'test_mode',
          display_name: 'Test Mode',
          description: '',
          system_prompt: '',
          temperature: 0.7,
          max_tokens: 8192,
          append_to_agent_system_prompt: false,
          append_to_agent_tools: true,
          display_order: 0,
        })
        expect(mockOnSave).toHaveBeenCalledWith(mockRouterMode)
        expect(mockOnClose).toHaveBeenCalled()
      })
    })

    it('handles 409 duplicate key error', async () => {
      const user = userEvent.setup()
      const conflictError = ApiError.http('/router-modes', 409, 'Conflict', {})
      mockCreateMode.mockRejectedValue(conflictError)

      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      const displayNameInput = screen.getByLabelText(/display name/i)

      await user.type(modeKeyInput, 'test_mode')
      await user.type(displayNameInput, 'Test Mode')

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(
        () => {
          const errorAlert = screen.getByRole('alert')
          expect(errorAlert).toHaveTextContent(/mode key already exists/i)
        },
        { timeout: 3000 },
      )
      expect(mockOnSave).not.toHaveBeenCalled()
      expect(mockOnClose).not.toHaveBeenCalled()
    })

    it('handles network error', async () => {
      const user = userEvent.setup()
      mockCreateMode.mockRejectedValue(new Error('Network error'))

      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const modeKeyInput = screen.getByLabelText(/mode key/i)
      const displayNameInput = screen.getByLabelText(/display name/i)

      await user.type(modeKeyInput, 'test_mode')
      await user.type(displayNameInput, 'Test Mode')

      const createButton = screen.getByRole('button', { name: /create/i })
      await user.click(createButton)

      await waitFor(() => {
        expect(screen.getByText('Network error')).toBeInTheDocument()
      })
    })
  })

  describe('submission - edit', () => {
    it('updates mode successfully', async () => {
      const user = userEvent.setup()
      const updated = { ...mockRouterMode, display_name: 'Updated Mode' }
      mockUpdateMode.mockResolvedValue(updated)

      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={mockRouterMode} routerId={routerId} />)

      const displayNameInput = screen.getByLabelText(/display name/i)
      await user.clear(displayNameInput)
      await user.type(displayNameInput, 'Updated Mode')

      const updateButton = screen.getByRole('button', { name: /update/i })
      await user.click(updateButton)

      await waitFor(() => {
        expect(mockUpdateMode).toHaveBeenCalledWith(mockRouterMode.id, {
          display_name: 'Updated Mode',
          description: mockRouterMode.description,
          system_prompt: mockRouterMode.system_prompt,
          temperature: mockRouterMode.temperature,
          max_tokens: mockRouterMode.max_tokens,
          append_to_agent_system_prompt: mockRouterMode.append_to_agent_system_prompt,
          append_to_agent_tools: mockRouterMode.append_to_agent_tools,
          display_order: mockRouterMode.display_order,
        })
        expect(mockOnSave).toHaveBeenCalledWith(updated)
        expect(mockOnClose).toHaveBeenCalled()
      })
    })
  })

  describe('cancel', () => {
    it('calls onClose when cancel is clicked', async () => {
      const user = userEvent.setup()
      render(<ModeFormDialog open={true} onClose={mockOnClose} onSave={mockOnSave} mode={null} routerId={routerId} />)

      const cancelButton = screen.getByRole('button', { name: /cancel/i })
      await user.click(cancelButton)

      expect(mockOnClose).toHaveBeenCalled()
    })
  })
})
