import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { SaveDiscardGroup } from './SaveDiscardGroup'

const { mockSelectDirty, mockRevertSteps } = vi.hoisted(() => ({
  mockSelectDirty: vi.fn(() => false),
  mockRevertSteps: vi.fn(() => Promise.resolve()),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectDirty) return mockSelectDirty()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectDirty: mockSelectDirty,
    revertSteps: mockRevertSteps,
  },
}))

const mockFlush = vi.fn()

beforeEach(() => {
  vi.clearAllMocks()
  mockSelectDirty.mockReturnValue(false)
  mockRevertSteps.mockReturnValue(Promise.resolve())
})

describe('SaveDiscardGroup', () => {
  it('Save and Discard buttons are disabled when not dirty', () => {
    render(<SaveDiscardGroup autoSaveFlush={mockFlush} autoSaveSaving={false} />)

    const saveBtn = screen.getByTestId('toolbar-save-button').querySelector('button')
    const discardBtn = screen.getByTestId('toolbar-discard-button').querySelector('button')

    expect(saveBtn).toBeDisabled()
    expect(discardBtn).toBeDisabled()
  })

  it('Save and Discard buttons are enabled when dirty', () => {
    mockSelectDirty.mockReturnValue(true)
    render(<SaveDiscardGroup autoSaveFlush={mockFlush} autoSaveSaving={false} />)

    const saveBtn = screen.getByTestId('toolbar-save-button').querySelector('button')
    const discardBtn = screen.getByTestId('toolbar-discard-button').querySelector('button')

    expect(saveBtn).not.toBeDisabled()
    expect(discardBtn).not.toBeDisabled()
  })

  it('Save calls autoSaveFlush', async () => {
    mockSelectDirty.mockReturnValue(true)
    const user = userEvent.setup()
    render(<SaveDiscardGroup autoSaveFlush={mockFlush} autoSaveSaving={false} />)

    await user.click(screen.getByText('Save'))

    expect(mockFlush).toHaveBeenCalledOnce()
  })

  it('Discard calls revertSteps', async () => {
    mockSelectDirty.mockReturnValue(true)
    const user = userEvent.setup()
    render(<SaveDiscardGroup autoSaveFlush={mockFlush} autoSaveSaving={false} />)

    await user.click(screen.getByText('Discard'))

    expect(mockRevertSteps).toHaveBeenCalledOnce()
  })

  it('shows Saving label when autoSaveSaving is true', () => {
    mockSelectDirty.mockReturnValue(true)
    render(<SaveDiscardGroup autoSaveFlush={mockFlush} autoSaveSaving={true} />)

    expect(screen.getByText('Saving')).toBeInTheDocument()
  })
})
