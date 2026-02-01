import { render, screen } from '@testing-library/react'
import { TasksPage } from './TasksPage'

describe('TasksPage', () => {
  it('renders tasks heading', () => {
    render(<TasksPage />)

    expect(screen.getByText('Tasks')).toBeInTheDocument()
  })
})
