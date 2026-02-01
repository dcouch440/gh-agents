import { render, screen } from '@testing-library/react'
import { SettingsPage } from './SettingsPage'

describe('SettingsPage', () => {
  it('renders settings heading', () => {
    render(<SettingsPage />)

    expect(screen.getByText('Settings')).toBeInTheDocument()
  })
})
