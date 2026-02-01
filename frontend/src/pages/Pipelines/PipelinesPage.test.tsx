import { render, screen } from '@testing-library/react'
import { PipelinesPage } from './PipelinesPage'

describe('PipelinesPage', () => {
  it('renders pipelines heading', () => {
    render(<PipelinesPage />)

    expect(screen.getByText('Pipelines')).toBeInTheDocument()
  })
})
