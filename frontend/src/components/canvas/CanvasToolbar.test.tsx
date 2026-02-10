import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { CanvasToolbar } from './CanvasToolbar'

const { mockSelectDirty, mockSelectSteps, mockSaveAllDirtySteps, mockRevertSteps } = vi.hoisted(() => ({
  mockSelectDirty: vi.fn(() => false),
  mockSelectSteps: vi.fn((): unknown[] => []),
  mockSaveAllDirtySteps: vi.fn(() => Promise.resolve()),
  mockRevertSteps: vi.fn(() => Promise.resolve()),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectDirty) return mockSelectDirty()
    if (selector === mockSelectSteps) return mockSelectSteps()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectDirty: mockSelectDirty,
    selectSteps: mockSelectSteps,
    selectActiveWorkflowId: vi.fn(),
    saveAllDirtySteps: mockSaveAllDirtySteps,
    revertSteps: mockRevertSteps,
  },
}))

const renderToolbar = () => render(<CanvasToolbar />)

beforeEach(() => {
  vi.clearAllMocks()
  mockSelectDirty.mockReturnValue(false)
})

describe('CanvasToolbar', () => {
  it('renders nothing when not dirty', () => {
    const { container } = renderToolbar()
    expect(container.firstChild).toBeNull()
  })

  it('renders Save and Discard buttons when dirty', () => {
    mockSelectDirty.mockReturnValue(true)
    renderToolbar()

    expect(screen.getByText('Save')).toBeInTheDocument()
    expect(screen.getByText('Discard')).toBeInTheDocument()
  })

  it('calls saveAllDirtySteps when Save is clicked', async () => {
    const user = userEvent.setup()
    mockSelectDirty.mockReturnValue(true)
    renderToolbar()

    await user.click(screen.getByText('Save'))

    expect(mockSaveAllDirtySteps).toHaveBeenCalledOnce()
  })

  it('calls revertSteps when Discard is clicked', async () => {
    const user = userEvent.setup()
    mockSelectDirty.mockReturnValue(true)
    renderToolbar()

    await user.click(screen.getByText('Discard'))

    expect(mockRevertSteps).toHaveBeenCalledOnce()
  })
})
