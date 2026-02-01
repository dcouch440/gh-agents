import { render, screen } from '@testing-library/react'
import { AgentsPage } from './AgentsPage'

describe('AgentsPage', () => {
  it('renders agents heading', () => {
    render(<AgentsPage />)

    expect(screen.getByText('Agents')).toBeInTheDocument()
  })
})
