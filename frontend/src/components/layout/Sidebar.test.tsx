import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Sidebar } from './Sidebar'

const mockToggle = vi.hoisted(() => vi.fn())
let mockCollapsed = false
let mockPendingCount = 0

vi.mock('@/hooks/useSidebar', () => ({
  useSidebar: () => ({ collapsed: mockCollapsed, toggle: mockToggle }),
}))

vi.mock('@/hooks/useNavigation', () => ({
  useNavigation: () => ({
    navItems: [
      { label: 'Dashboard', path: '/', icon: 'icon-dash', isActive: true },
      { label: 'Review Queue', path: '/review-queue', icon: 'icon-review', isActive: false },
    ],
  }),
}))

vi.mock('@/stores', () => ({
  useStore: () => mockPendingCount,
  reviewQueueStore: {
    store: { getState: () => ({}), subscribe: () => () => {} },
    selectPendingCount: () => mockPendingCount,
  },
}))

vi.mock('./SidebarNavItem', () => ({
  SidebarNavItem: function SidebarNavItem(props: { label: string; badge?: number }) {
    return <div data-testid={`nav-item-${props.label}`} data-badge={props.badge !== undefined ? String(props.badge) : ''} />
  },
}))

vi.mock('./ThemeToggle', () => ({
  ThemeToggle: function ThemeToggle() {
    return <div data-testid="theme-toggle" />
  },
}))

describe('Sidebar', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockCollapsed = false
    mockPendingCount = 0
  })

  it('renders app name when expanded', () => {
    render(<Sidebar />)
    expect(screen.getByText('nexor')).toBeInTheDocument()
  })

  it('renders abbreviated name when collapsed', () => {
    mockCollapsed = true
    render(<Sidebar />)
    expect(screen.getByText('n')).toBeInTheDocument()
    expect(screen.queryByText('nexor')).not.toBeInTheDocument()
  })

  it('renders nav items', () => {
    render(<Sidebar />)
    expect(screen.getByTestId('nav-item-Dashboard')).toBeInTheDocument()
    expect(screen.getByTestId('nav-item-Review Queue')).toBeInTheDocument()
  })

  it('renders theme toggle', () => {
    render(<Sidebar />)
    expect(screen.getByTestId('theme-toggle')).toBeInTheDocument()
  })

  it('calls toggle when collapse button is clicked', async () => {
    const user = userEvent.setup()
    render(<Sidebar />)

    const collapseButton = screen.getByLabelText(/Collapse sidebar/)
    await user.click(collapseButton)
    expect(mockToggle).toHaveBeenCalledOnce()
  })

  it('passes pending count as badge to Review Queue nav item', () => {
    mockPendingCount = 5
    render(<Sidebar />)
    const reviewItem = screen.getByTestId('nav-item-Review Queue')
    expect(reviewItem).toHaveAttribute('data-badge', '5')
  })
})
