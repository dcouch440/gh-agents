import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { SaveDiscardGroup } from './SaveDiscardGroup'

const { mockSelectDirty, mockSaveAllDirtySteps, mockRevertSteps } = vi.hoisted(() => ({
  mockSelectDirty: vi.fn(() => false),
  mockSaveAllDirtySteps: vi.fn(() => Promise.resolve()),
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
    saveAllDirtySteps: mockSaveAllDirtySteps,
    revertSteps: mockRevertSteps,
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  mockSelectDirty.mockReturnValue(false)
  mockSaveAllDirtySteps.mockReturnValue(Promise.resolve())
  mockRevertSteps.mockReturnValue(Promise.resolve())
})

describe('SaveDiscardGroup', () => {
  it('Save and Discard buttons are disabled when not dirty', () => {
    render(<SaveDiscardGroup />)

    const saveBtn = screen.getByTestId('toolbar-save-button').querySelector('button')
    const discardBtn = screen.getByTestId('toolbar-discard-button').querySelector('button')

    expect(saveBtn).toBeDisabled()
    expect(discardBtn).toBeDisabled()
  })

  it('Save and Discard buttons are enabled when dirty', () => {
    mockSelectDirty.mockReturnValue(true)
    render(<SaveDiscardGroup />)

    const saveBtn = screen.getByTestId('toolbar-save-button').querySelector('button')
    const discardBtn = screen.getByTestId('toolbar-discard-button').querySelector('button')

    expect(saveBtn).not.toBeDisabled()
    expect(discardBtn).not.toBeDisabled()
  })

  it('Save calls saveAllDirtySteps', async () => {
    mockSelectDirty.mockReturnValue(true)
    const user = userEvent.setup()
    render(<SaveDiscardGroup />)

    await user.click(screen.getByText('Save'))

    expect(mockSaveAllDirtySteps).toHaveBeenCalledOnce()
  })

  it('Discard calls revertSteps', async () => {
    mockSelectDirty.mockReturnValue(true)
    const user = userEvent.setup()
    render(<SaveDiscardGroup />)

    await user.click(screen.getByText('Discard'))

    expect(mockRevertSteps).toHaveBeenCalledOnce()
  })
})
