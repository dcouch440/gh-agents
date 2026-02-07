import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ReviewQueueNotification } from './ReviewQueueNotification'

const mockDismiss = vi.hoisted(() => vi.fn())
let mockNotification: { id: string; message: string } | null = null

vi.mock('@/stores', () => ({
  useStore: (_store: unknown, selector: () => unknown) => selector(),
  reviewQueueStore: {
    store: { getState: () => ({}), subscribe: () => () => {} },
    selectNotification: () => mockNotification,
    dismissNotification: mockDismiss,
  },
}))

describe('ReviewQueueNotification', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockNotification = null
  })

  it('renders nothing visible when notification is null', () => {
    const { container } = render(<ReviewQueueNotification />)
    // Snackbar should not be visible (MUI keeps it in DOM but hidden)
    expect(container.querySelector('.MuiSnackbar-root')).not.toBeInTheDocument()
  })

  it('renders notification message when present', () => {
    mockNotification = { id: 'n1', message: 'New execution awaiting review' }
    render(<ReviewQueueNotification />)
    expect(screen.getByText('New execution awaiting review')).toBeInTheDocument()
  })

  it('renders as warning severity', () => {
    mockNotification = { id: 'n1', message: 'Test message' }
    render(<ReviewQueueNotification />)
    expect(screen.getByRole('alert')).toBeInTheDocument()
  })
})
