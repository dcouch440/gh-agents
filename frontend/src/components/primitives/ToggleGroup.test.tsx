import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ToggleGroup } from './ToggleGroup'

const options = [
  { value: 'a', label: 'Alpha' },
  { value: 'b', label: 'Beta' },
  { value: 'c', label: 'Gamma' },
]

describe('ToggleGroup', () => {
  it('renders all options as buttons', () => {
    render(<ToggleGroup options={options} value="a" onChange={vi.fn()} />)
    expect(screen.getAllByRole('button')).toHaveLength(3)
    expect(screen.getByText('Alpha')).toBeInTheDocument()
    expect(screen.getByText('Beta')).toBeInTheDocument()
    expect(screen.getByText('Gamma')).toBeInTheDocument()
  })

  it('active option has active class', () => {
    render(<ToggleGroup options={options} value="b" onChange={vi.fn()} />)
    expect(screen.getByText('Beta')).toHaveClass('toggle-group__btn--active')
    expect(screen.getByText('Alpha')).not.toHaveClass('toggle-group__btn--active')
  })

  it('clicking calls onChange with value', () => {
    const onChange = vi.fn()
    render(<ToggleGroup options={options} value="a" onChange={onChange} />)

    fireEvent.click(screen.getByText('Gamma'))
    expect(onChange).toHaveBeenCalledWith('c')
  })

  it('applies className', () => {
    const { container } = render(
      <ToggleGroup options={options} value="a" onChange={vi.fn()} className="extra" />
    )
    expect(container.firstChild).toHaveClass('toggle-group', 'extra')
  })
})
