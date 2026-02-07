import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PropertySection } from './PropertySection'

describe('PropertySection', () => {
  it('renders title and children', () => {
    render(
      <PropertySection title="Settings">
        <span>Content</span>
      </PropertySection>
    )
    expect(screen.getByText('Settings')).toBeInTheDocument()
    expect(screen.getByText('Content')).toBeInTheDocument()
  })

  it('renders without title when null', () => {
    render(
      <PropertySection title={null}>
        <span>Content</span>
      </PropertySection>
    )
    expect(screen.getByText('Content')).toBeInTheDocument()
    expect(screen.queryByText('Settings')).not.toBeInTheDocument()
  })

  it('shows chevron when collapsible', () => {
    const { container } = render(
      <PropertySection title="Config" onToggle={vi.fn()}>
        <span>Content</span>
      </PropertySection>
    )
    expect(container.querySelector('[data-testid="KeyboardArrowDownRoundedIcon"]')).toBeInTheDocument()
  })

  it('does not show chevron when not collapsible', () => {
    const { container } = render(
      <PropertySection title="Config">
        <span>Content</span>
      </PropertySection>
    )
    expect(container.querySelector('[data-testid="KeyboardArrowDownRoundedIcon"]')).not.toBeInTheDocument()
  })

  it('calls onToggle when title is clicked', async () => {
    const user = userEvent.setup()
    const onToggle = vi.fn()
    render(
      <PropertySection title="Config" onToggle={onToggle}>
        <span>Content</span>
      </PropertySection>
    )

    await user.click(screen.getByText('Config'))
    expect(onToggle).toHaveBeenCalledOnce()
  })

  it('wraps children in Collapse when collapsible', () => {
    const { container } = render(
      <PropertySection title="Config" open={false} onToggle={vi.fn()}>
        <span>Hidden Content</span>
      </PropertySection>
    )
    const collapse = container.querySelector('.MuiCollapse-root')
    expect(collapse).toBeInTheDocument()
  })
})
