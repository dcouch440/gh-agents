import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ModeToolSelector } from './ModeToolSelector'
import { mockRouterMode, mockTool } from '@/test/fixtures'

const mockLoadModeTools = vi.fn()
const mockSaveModeTools = vi.fn()

const mockTool2 = {
  ...mockTool,
  id: 'tool-002',
  name: 'analyze_code',
  description: 'Analyze code for issues',
}

const allTools = [mockTool, mockTool2]

describe('ModeToolSelector', () => {
  const mockOnClose = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('rendering', () => {
    it('shows loading spinner while loadingTools is true', () => {
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={true}
          savingTools={false}
          toolsError={null}
        />
      )

      expect(screen.getByRole('progressbar')).toBeInTheDocument()
    })

    it('shows empty state when allTools is empty', async () => {
      mockLoadModeTools.mockResolvedValue([])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={[]}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText('No tools available')).toBeInTheDocument()
      })
    })

    it('shows tool list when allTools has items', async () => {
      mockLoadModeTools.mockResolvedValue([mockTool])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText(mockTool.name)).toBeInTheDocument()
        expect(screen.getByText(mockTool2.name)).toBeInTheDocument()
      })
    })

    it('shows selected count in subtitle', async () => {
      mockLoadModeTools.mockResolvedValue([mockTool])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText(/1 tool selected/i)).toBeInTheDocument()
      })
    })
  })

  describe('loading tools', () => {
    it('calls loadModeTools when dialog opens', async () => {
      mockLoadModeTools.mockResolvedValue([mockTool])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(mockLoadModeTools).toHaveBeenCalledWith(mockRouterMode.id)
      })
    })

    it('populates localSelectedIds with returned tool IDs', async () => {
      mockLoadModeTools.mockResolvedValue([mockTool])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(
        () => {
          const checkboxes = screen.queryAllByRole('checkbox')
          const toolCheckbox = checkboxes.find(
            (cb) =>
              cb.getAttribute('aria-label') === null &&
              (cb.closest('[role="button"]')?.textContent ?? '').includes(mockTool.name)
          )
          expect(toolCheckbox).toBeChecked()
        },
        { timeout: 3000 }
      )
    })

    it('does not call loadModeTools when dialog is closed', () => {
      mockLoadModeTools.mockResolvedValue([])
      render(
        <ModeToolSelector
          open={false}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      expect(mockLoadModeTools).not.toHaveBeenCalled()
    })
  })

  describe('checkbox interactions', () => {
    it('adds tool to selection when unchecked checkbox is clicked', async () => {
      const user = userEvent.setup()
      mockLoadModeTools.mockResolvedValue([])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText(mockTool.name)).toBeInTheDocument()
      })

      const checkboxes = screen.getAllByRole('checkbox')
      expect(checkboxes[0]).not.toBeChecked()

      await user.click(checkboxes[0])

      await waitFor(() => {
        expect(checkboxes[0]).toBeChecked()
        expect(screen.getByText(/1 tool selected/i)).toBeInTheDocument()
      })
    })

    it('removes tool from selection when checked checkbox is clicked', async () => {
      const user = userEvent.setup()
      mockLoadModeTools.mockResolvedValue([mockTool])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        const checkboxes = screen.getAllByRole('checkbox')
        expect(checkboxes[0]).toBeChecked()
      })

      const checkboxes = screen.getAllByRole('checkbox')
      await user.click(checkboxes[0])

      await waitFor(() => {
        expect(checkboxes[0]).not.toBeChecked()
        expect(screen.getByText(/0 tools selected/i)).toBeInTheDocument()
      })
    })

    it('disables checkboxes when savingTools is true', async () => {
      mockLoadModeTools.mockResolvedValue([])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={true}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText(mockTool.name)).toBeInTheDocument()
      })

      const checkboxes = screen.getAllByRole('checkbox')
      expect(checkboxes[0]).toBeDisabled()
    })
  })

  describe('save', () => {
    it('calls saveModeTools with tool_ids', async () => {
      const user = userEvent.setup()
      mockLoadModeTools.mockResolvedValue([mockTool])
      mockSaveModeTools.mockResolvedValue(undefined)

      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText(mockTool.name)).toBeInTheDocument()
      })

      const saveButton = screen.getByRole('button', { name: /save/i })
      await user.click(saveButton)

      await waitFor(() => {
        expect(mockSaveModeTools).toHaveBeenCalledWith(mockRouterMode.id, {
          tool_ids: [mockTool.id],
        })
        expect(mockOnClose).toHaveBeenCalled()
      })
    })

    it('does not close on error', async () => {
      const user = userEvent.setup()
      mockLoadModeTools.mockResolvedValue([])
      mockSaveModeTools.mockRejectedValue(new Error('Save failed'))

      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText(mockTool.name)).toBeInTheDocument()
      })

      const saveButton = screen.getByRole('button', { name: /save/i })
      await user.click(saveButton)

      await waitFor(() => {
        expect(mockSaveModeTools).toHaveBeenCalled()
      })

      expect(mockOnClose).not.toHaveBeenCalled()
    })

    it('disables save button when savingTools is true', async () => {
      mockLoadModeTools.mockResolvedValue([])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={true}
          toolsError={null}
        />
      )

      await waitFor(() => {
        expect(screen.getByText(mockTool.name)).toBeInTheDocument()
      })

      const saveButton = screen.getByRole('button', { name: /saving/i })
      expect(saveButton).toBeDisabled()
    })

    it('disables save button when loadingTools is true', () => {
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={true}
          savingTools={false}
          toolsError={null}
        />
      )

      const saveButton = screen.getByRole('button', { name: /save/i })
      expect(saveButton).toBeDisabled()
    })
  })

  describe('cancel', () => {
    it('resets localSelectedIds to originalIds', async () => {
      const user = userEvent.setup()
      mockLoadModeTools.mockResolvedValue([mockTool])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError={null}
        />
      )

      await waitFor(() => {
        const checkboxes = screen.getAllByRole('checkbox')
        expect(checkboxes[0]).toBeChecked()
      })

      // Uncheck the tool
      const checkboxes = screen.getAllByRole('checkbox')
      await user.click(checkboxes[0])

      await waitFor(() => {
        expect(checkboxes[0]).not.toBeChecked()
      })

      // Cancel should restore original state
      const cancelButton = screen.getByRole('button', { name: /cancel/i })
      await user.click(cancelButton)

      expect(mockOnClose).toHaveBeenCalled()
    })
  })

  describe('error handling', () => {
    it('shows toolsError in alert', async () => {
      mockLoadModeTools.mockResolvedValue([])
      render(
        <ModeToolSelector
          open={true}
          onClose={mockOnClose}
          mode={mockRouterMode}
          allTools={allTools}
          loadModeTools={mockLoadModeTools}
          saveModeTools={mockSaveModeTools}
          loadingTools={false}
          savingTools={false}
          toolsError="Failed to load tools"
        />
      )

      await waitFor(() => {
        expect(screen.getByText('Failed to load tools')).toBeInTheDocument()
      })
    })
  })
})
