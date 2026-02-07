import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PanelToggle } from './PanelToggle'

describe('PanelToggle', () => {
  it('renders a switch element', () => {
    render(<PanelToggle checked={false} onChange={vi.fn()} />)
    expect(screen.getByRole('switch')).toBeInTheDocument()
  })

  it('reflects checked state', () => {
    render(<PanelToggle checked={true} onChange={vi.fn()} />)
    expect(screen.getByRole('switch')).toBeChecked()
  })

  it('calls onChange with boolean value on click', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    render(<PanelToggle checked={false} onChange={onChange} />)

    await user.click(screen.getByRole('switch'))
    expect(onChange).toHaveBeenCalledWith(true)
  })
})
