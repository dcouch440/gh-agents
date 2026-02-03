import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { AgentWorkshopPage } from './AgentWorkshopPage'

const mockNavigate = vi.fn()
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom')
  return { ...actual, useNavigate: () => mockNavigate }
})

const mockCreate = vi.hoisted(() => vi.fn())
vi.mock('@/api', () => ({
  api: {
    agents: {
      create: mockCreate,
    },
  },
}))

vi.mock('@/components/primitives/CodeEditor', () => ({
  CodeEditor: ({ value, onChange, placeholder }: { value: string; onChange: (v: string) => void; placeholder?: string }) => (
    <textarea
      data-testid="code-editor"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
    />
  ),
}))

const renderPage = () =>
  render(
    <MemoryRouter>
      <AgentWorkshopPage />
    </MemoryRouter>
  )

describe('AgentWorkshopPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders page header and split layout', () => {
    renderPage()
    expect(screen.getByText('Agent Workshop')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Agent name...')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Save' })).toBeInTheDocument()
  })

  it('renders editor toggle group', () => {
    renderPage()
    expect(screen.getByRole('button', { name: 'Edit' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Preview' })).toBeInTheDocument()
  })

  it('renders model config fields', () => {
    renderPage()
    expect(screen.getByLabelText('Model')).toBeInTheDocument()
    expect(screen.getByLabelText('Max Tokens')).toBeInTheDocument()
    expect(screen.getByLabelText('Temperature')).toBeInTheDocument()
  })

  it('disables save when name is empty', () => {
    renderPage()
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled()
  })

  it('enables save when name is filled', () => {
    renderPage()
    fireEvent.change(screen.getByPlaceholderText('Agent name...'), { target: { value: 'MyAgent' } })
    expect(screen.getByRole('button', { name: 'Save' })).not.toBeDisabled()
  })

  it('calls api.agents.create on save and navigates', async () => {
    mockCreate.mockResolvedValueOnce({ id: 'new-agent' })
    renderPage()

    fireEvent.change(screen.getByPlaceholderText('Agent name...'), { target: { value: 'MyAgent' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(mockCreate).toHaveBeenCalledWith(
        expect.objectContaining({ name: 'MyAgent', model_id: 'sonnet' })
      )
    })

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/agents')
    })
  })

  it('displays error on save failure', async () => {
    mockCreate.mockRejectedValueOnce(new Error('Server error'))
    renderPage()

    fireEvent.change(screen.getByPlaceholderText('Agent name...'), { target: { value: 'MyAgent' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(screen.getByText('Server error')).toBeInTheDocument()
    })
  })

  it('shows chat empty state', () => {
    renderPage()
    expect(screen.getByText('No messages yet')).toBeInTheDocument()
  })

  it('toggles between edit and preview mode', () => {
    renderPage()
    expect(screen.getByTestId('code-editor')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Preview' }))
    expect(screen.queryByTestId('code-editor')).not.toBeInTheDocument()
  })
})
