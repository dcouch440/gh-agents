import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { OptionTray } from './OptionTray'

const { mockSelectDirty, mockSelectActiveWorkflowId, mockSelectSteps, mockSaveAllDirtySteps, mockRevertSteps } = vi.hoisted(() => ({
  mockSelectDirty: vi.fn(() => false),
  mockSelectActiveWorkflowId: vi.fn(() => 'wf-001'),
  mockSelectSteps: vi.fn((): unknown[] => []),
  mockSaveAllDirtySteps: vi.fn(() => Promise.resolve()),
  mockRevertSteps: vi.fn(() => Promise.resolve()),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectDirty) return mockSelectDirty()
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    if (selector === mockSelectSteps) return mockSelectSteps()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectDirty: mockSelectDirty,
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
    selectSteps: mockSelectSteps,
    saveAllDirtySteps: mockSaveAllDirtySteps,
    revertSteps: mockRevertSteps,
  },
}))

vi.mock('framer-motion', () => ({
  AnimatePresence: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  motion: {
    div: ({ children, style }: Record<string, unknown>) => <div style={style as React.CSSProperties}>{children as React.ReactNode}</div>,
  },
}))

const renderTray = () => render(<OptionTray />)

beforeEach(() => {
  vi.clearAllMocks()
  mockSelectDirty.mockReturnValue(false)
  mockSelectActiveWorkflowId.mockReturnValue('wf-001')
})

describe('OptionTray', () => {
  it('renders nothing when no activeWorkflowId', () => {
    mockSelectActiveWorkflowId.mockReturnValue(null)
    const { container } = renderTray()
    expect(container.innerHTML).toBe('')
  })

  it('renders toggle pill when activeWorkflowId exists', () => {
    renderTray()
    expect(screen.getByTestId('tray-toggle')).toBeInTheDocument()
  })

  it('shows all buttons when toggle is clicked', async () => {
    const user = userEvent.setup()
    renderTray()

    await user.click(screen.getByTestId('tray-toggle'))

    expect(screen.getByText('Run')).toBeInTheDocument()
    expect(screen.getByText('Saved')).toBeInTheDocument()
    expect(screen.getByText('Discard')).toBeInTheDocument()
  })

  it('Save shows "Saved" and is disabled when not dirty', async () => {
    const user = userEvent.setup()
    renderTray()

    await user.click(screen.getByTestId('tray-toggle'))

    expect(screen.getByText('Saved')).toBeInTheDocument()
    expect(screen.getByTestId('toolbar-save-button').querySelector('button')).toBeDisabled()
  })

  it('Discard is disabled when not dirty', async () => {
    const user = userEvent.setup()
    renderTray()

    await user.click(screen.getByTestId('tray-toggle'))

    expect(screen.getByTestId('toolbar-discard-button').querySelector('button')).toBeDisabled()
  })

  it('Save shows "Save" and is enabled when dirty', async () => {
    const user = userEvent.setup()
    mockSelectDirty.mockReturnValue(true)
    renderTray()

    await user.click(screen.getByTestId('tray-toggle'))

    expect(screen.getByText('Save')).toBeInTheDocument()
    expect(screen.getByTestId('toolbar-save-button').querySelector('button')).not.toBeDisabled()
  })

  it('calls saveAllDirtySteps when Save is clicked', async () => {
    const user = userEvent.setup()
    mockSelectDirty.mockReturnValue(true)
    renderTray()

    await user.click(screen.getByTestId('tray-toggle'))
    await user.click(screen.getByText('Save'))

    expect(mockSaveAllDirtySteps).toHaveBeenCalledOnce()
  })

  it('calls revertSteps when Discard is clicked', async () => {
    const user = userEvent.setup()
    mockSelectDirty.mockReturnValue(true)
    renderTray()

    await user.click(screen.getByTestId('tray-toggle'))
    await user.click(screen.getByText('Discard'))

    expect(mockRevertSteps).toHaveBeenCalledOnce()
  })

  it('toggle closes the tray', async () => {
    const user = userEvent.setup()
    renderTray()

    await user.click(screen.getByTestId('tray-toggle'))
    expect(screen.getByText('Run')).toBeInTheDocument()

    await user.click(screen.getByTestId('tray-toggle'))
    expect(screen.queryByText('Run')).not.toBeInTheDocument()
  })
})
