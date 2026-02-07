import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TabSelector } from './TabSelector'

const options = [
  { value: 'decomp', label: 'Decomp' },
  { value: 'route', label: 'Route' },
  { value: 'review', label: 'Review' },
]

describe('TabSelector', () => {
  it('renders all tab options', () => {
    render(<TabSelector options={options} value="decomp" onChange={vi.fn()} />)
    expect(screen.getByText('Decomp')).toBeInTheDocument()
    expect(screen.getByText('Route')).toBeInTheDocument()
    expect(screen.getByText('Review')).toBeInTheDocument()
  })

  it('marks the active tab with aria-selected', () => {
    render(<TabSelector options={options} value="route" onChange={vi.fn()} />)
    const routeTab = screen.getByRole('tab', { name: 'Route' })
    expect(routeTab).toHaveAttribute('aria-selected', 'true')

    const decompTab = screen.getByRole('tab', { name: 'Decomp' })
    expect(decompTab).toHaveAttribute('aria-selected', 'false')
  })

  it('calls onChange when a tab is clicked', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    render(<TabSelector options={options} value="decomp" onChange={onChange} />)

    await user.click(screen.getByText('Review'))
    expect(onChange).toHaveBeenCalledWith('review')
  })
})
