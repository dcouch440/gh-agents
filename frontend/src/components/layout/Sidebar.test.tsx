import { render, screen } from '@testing-library/react'
import { BrowserRouter } from 'react-router-dom'
import { Sidebar } from './Sidebar'

describe('Sidebar', () => {
  it('renders app name', () => {
    render(
      <BrowserRouter>
        <Sidebar />
      </BrowserRouter>,
    )

    expect(screen.getByText('nexor')).toBeInTheDocument()
  })

  it('renders all navigation links', () => {
    render(
      <BrowserRouter>
        <Sidebar />
      </BrowserRouter>,
    )

    expect(screen.getByText('Dashboard')).toBeInTheDocument()
    expect(screen.getByText('Chat')).toBeInTheDocument()
    expect(screen.getByText('Agents')).toBeInTheDocument()
    expect(screen.getByText('Pipelines')).toBeInTheDocument()
    expect(screen.getByText('Tasks')).toBeInTheDocument()
    expect(screen.getByText('Documents')).toBeInTheDocument()
    expect(screen.getByText('Settings')).toBeInTheDocument()
    expect(screen.getByText('Showcase')).toBeInTheDocument()
  })

  it('links have correct href attributes', () => {
    render(
      <BrowserRouter>
        <Sidebar />
      </BrowserRouter>,
    )

    expect(screen.getByText('Dashboard').closest('a')).toHaveAttribute('href', '/')
    expect(screen.getByText('Chat').closest('a')).toHaveAttribute('href', '/chat')
    expect(screen.getByText('Agents').closest('a')).toHaveAttribute('href', '/agents')
    expect(screen.getByText('Pipelines').closest('a')).toHaveAttribute('href', '/pipelines')
    expect(screen.getByText('Tasks').closest('a')).toHaveAttribute('href', '/tasks')
    expect(screen.getByText('Documents').closest('a')).toHaveAttribute('href', '/documents')
    expect(screen.getByText('Settings').closest('a')).toHaveAttribute('href', '/settings')
    expect(screen.getByText('Showcase').closest('a')).toHaveAttribute('href', '/showcase')
  })
})
