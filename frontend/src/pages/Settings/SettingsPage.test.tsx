import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { SettingsPage } from './SettingsPage'

vi.mock('./RouterModesTab', () => ({
  RouterModesTab: () => <div data-testid="router-modes-tab">Router Modes Tab</div>,
}))

const renderPage = () =>
  render(
    <MemoryRouter>
      <SettingsPage />
    </MemoryRouter>,
  )

describe('SettingsPage', () => {
  it('renders settings heading', () => {
    renderPage()

    expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument()
  })

  it('renders with overview tab selected by default', () => {
    renderPage()

    const overviewTab = screen.getByRole('tab', { name: /overview/i })
    expect(overviewTab).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByText(/general settings and configuration options/i)).toBeInTheDocument()
  })

  it('shows two tabs', () => {
    renderPage()

    expect(screen.getByRole('tab', { name: /overview/i })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /router modes/i })).toBeInTheDocument()
  })

  it('switches to router modes tab when clicked', async () => {
    const user = userEvent.setup()
    renderPage()

    const routerModesTab = screen.getByRole('tab', { name: /router modes/i })
    await user.click(routerModesTab)

    expect(routerModesTab).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByTestId('router-modes-tab')).toBeInTheDocument()
  })

  it('switches back to overview tab when clicked', async () => {
    const user = userEvent.setup()
    renderPage()

    const routerModesTab = screen.getByRole('tab', { name: /router modes/i })
    const overviewTab = screen.getByRole('tab', { name: /overview/i })

    await user.click(routerModesTab)
    expect(routerModesTab).toHaveAttribute('aria-selected', 'true')

    await user.click(overviewTab)
    expect(overviewTab).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByText(/general settings and configuration options/i)).toBeInTheDocument()
  })
})
