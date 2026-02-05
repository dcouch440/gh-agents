import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SettingsPage } from './SettingsPage'

vi.mock('./RouterModesTab', () => ({
  RouterModesTab: () => <div data-testid="router-modes-tab">Router Modes Tab</div>,
}))

describe('SettingsPage', () => {
  it('renders settings heading', () => {
    render(<SettingsPage />)

    expect(screen.getByText('Settings')).toBeInTheDocument()
  })

  it('renders with overview tab selected by default', () => {
    render(<SettingsPage />)

    const overviewTab = screen.getByRole('tab', { name: /overview/i })
    expect(overviewTab).toHaveAttribute('aria-selected', 'true')
    expect(
      screen.getByText(/general settings and configuration options/i)
    ).toBeInTheDocument()
  })

  it('shows two tabs', () => {
    render(<SettingsPage />)

    expect(screen.getByRole('tab', { name: /overview/i })).toBeInTheDocument()
    expect(
      screen.getByRole('tab', { name: /router modes/i })
    ).toBeInTheDocument()
  })

  it('switches to router modes tab when clicked', async () => {
    const user = userEvent.setup()
    render(<SettingsPage />)

    const routerModesTab = screen.getByRole('tab', { name: /router modes/i })
    await user.click(routerModesTab)

    expect(routerModesTab).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByTestId('router-modes-tab')).toBeInTheDocument()
  })

  it('switches back to overview tab when clicked', async () => {
    const user = userEvent.setup()
    render(<SettingsPage />)

    const routerModesTab = screen.getByRole('tab', { name: /router modes/i })
    const overviewTab = screen.getByRole('tab', { name: /overview/i })

    await user.click(routerModesTab)
    expect(routerModesTab).toHaveAttribute('aria-selected', 'true')

    await user.click(overviewTab)
    expect(overviewTab).toHaveAttribute('aria-selected', 'true')
    expect(
      screen.getByText(/general settings and configuration options/i)
    ).toBeInTheDocument()
  })
})
