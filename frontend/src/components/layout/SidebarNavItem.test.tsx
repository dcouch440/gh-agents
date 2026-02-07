import { type ReactNode } from 'react'
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import DashboardIcon from '@mui/icons-material/Dashboard'
import { SidebarNavItem } from './SidebarNavItem'

type Props = {
  label: string
  path: string
  icon: ReactNode
  isActive: boolean
  collapsed: boolean
  badge?: number
}

const defaultProps: Props = {
  label: 'Dashboard',
  path: '/dashboard',
  icon: <DashboardIcon />,
  isActive: false,
  collapsed: false,
}

const renderItem = (overrides: Partial<Props> = {}) =>
  render(
    <MemoryRouter>
      <SidebarNavItem {...defaultProps} {...overrides} />
    </MemoryRouter>,
  )

describe('SidebarNavItem', () => {
  it('renders label when not collapsed', () => {
    renderItem()
    expect(screen.getByText('Dashboard')).toBeInTheDocument()
  })

  it('hides label when collapsed', () => {
    renderItem({ collapsed: true })
    expect(screen.queryByText('Dashboard')).not.toBeInTheDocument()
  })

  it('links to the correct path', () => {
    renderItem()
    const link = screen.getByRole('link')
    expect(link).toHaveAttribute('href', '/dashboard')
  })

  it('renders badge count when badge is provided and not collapsed', () => {
    renderItem({ badge: 3 })
    expect(screen.getByText('3')).toBeInTheDocument()
  })

  it('renders dot badge when collapsed with badge', () => {
    renderItem({ collapsed: true, badge: 5 })
    // Dot variant doesn't render the number
    expect(screen.queryByText('5')).not.toBeInTheDocument()
    // But the badge element exists
    expect(document.querySelector('.MuiBadge-dot')).toBeInTheDocument()
  })

  it('does not render badge when badge is undefined', () => {
    renderItem()
    expect(document.querySelector('.MuiBadge-root')).not.toBeInTheDocument()
  })
})
