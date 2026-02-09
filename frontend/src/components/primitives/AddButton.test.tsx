import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { AddButton } from './AddButton'

describe('AddButton', () => {
  it('renders the label', () => {
    render(<AddButton label="Add Step" onClick={vi.fn()} />)
    expect(screen.getByText('Add Step')).toBeInTheDocument()
  })

  it('calls onClick when clicked', async () => {
    const user = userEvent.setup()
    const onClick = vi.fn()
    render(<AddButton label="Add" onClick={onClick} />)

    await user.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledOnce()
  })

  it('renders custom icon when provided', () => {
    render(
      <AddButton
        label="Custom"
        onClick={vi.fn()}
        icon={<span data-testid="custom-icon">+</span>}
      />
    )
    expect(screen.getByTestId('custom-icon')).toBeInTheDocument()
  })

  it('renders default icon when no icon provided', () => {
    const { container } = render(<AddButton label="Add" onClick={vi.fn()} />)
    const svg = container.querySelector('svg')
    expect(svg).toBeInTheDocument()
  })
})
