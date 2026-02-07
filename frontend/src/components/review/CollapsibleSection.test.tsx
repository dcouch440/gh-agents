import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CollapsibleSection } from './CollapsibleSection'

describe('CollapsibleSection', () => {
  it('renders title text', () => {
    render(
      <CollapsibleSection title="Input" open={true} onToggle={vi.fn()}>
        <div>Content</div>
      </CollapsibleSection>,
    )
    expect(screen.getByText('Input')).toBeInTheDocument()
  })

  it('shows down-arrow when open', () => {
    render(
      <CollapsibleSection title="Input" open={true} onToggle={vi.fn()}>
        <div>Content</div>
      </CollapsibleSection>,
    )
    expect(screen.getByText('\u25BC')).toBeInTheDocument()
  })

  it('shows right-arrow when closed', () => {
    render(
      <CollapsibleSection title="Input" open={false} onToggle={vi.fn()}>
        <div>Content</div>
      </CollapsibleSection>,
    )
    expect(screen.getByText('\u25B6')).toBeInTheDocument()
  })

  it('renders children when open', () => {
    render(
      <CollapsibleSection title="Input" open={true} onToggle={vi.fn()}>
        <div>Visible Content</div>
      </CollapsibleSection>,
    )
    expect(screen.getByText('Visible Content')).toBeInTheDocument()
  })

  it('calls onToggle when header is clicked', async () => {
    const user = userEvent.setup()
    const onToggle = vi.fn()
    render(
      <CollapsibleSection title="Input" open={true} onToggle={onToggle}>
        <div>Content</div>
      </CollapsibleSection>,
    )

    await user.click(screen.getByText('Input'))
    expect(onToggle).toHaveBeenCalledOnce()
  })
})
