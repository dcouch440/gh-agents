import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeProvider, createTheme } from '@mui/material'
import { CanvasToolbar } from './CanvasToolbar'

const theme = createTheme({ palette: { mode: 'dark' } })

const {
  mockSelectDirty,
  mockSaveAllDirtySteps,
  mockRevertSteps,
} = vi.hoisted(() => ({
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

const renderToolbar = () =>
  render(
    <ThemeProvider theme={theme}>
      <CanvasToolbar />
    </ThemeProvider>,
  )

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
