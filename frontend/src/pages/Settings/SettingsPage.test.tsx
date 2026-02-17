import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { SettingsPage } from './SettingsPage'

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

  it('renders overview content', () => {
    renderPage()

    expect(screen.getByText(/general settings and configuration options/i)).toBeInTheDocument()
  })
})
