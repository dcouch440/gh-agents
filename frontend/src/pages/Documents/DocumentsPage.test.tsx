import { render, screen } from '@testing-library/react'
import { DocumentsPage } from './DocumentsPage'

describe('DocumentsPage', () => {
  it('renders documents heading', () => {
    render(<DocumentsPage />)

    expect(screen.getByText('Documents')).toBeInTheDocument()
  })
})
