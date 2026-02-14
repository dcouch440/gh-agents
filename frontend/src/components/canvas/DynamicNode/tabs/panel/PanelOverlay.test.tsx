import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { PanelOverlay } from './PanelOverlay'

const SIMPLE_CONTENT = [
  '# Test Panel',
  '- [ ] Option A',
  '- [ ] Option B',
  '- Informational bullet',
].join('\n')

describe('PanelOverlay', () => {
  it('renders panel content with sections', () => {
    render(
      <PanelOverlay
        content={SIMPLE_CONTENT}
        submitLabel="Submit"
        onSubmit={vi.fn()}
        onDismiss={vi.fn()}
      />,
    )

    expect(screen.getByText('Test Panel')).toBeInTheDocument()
    expect(screen.getByText('Option A')).toBeInTheDocument()
    expect(screen.getByText('Option B')).toBeInTheDocument()
    // Plain bullet rendered as body markdown
    expect(screen.getByText('Informational bullet')).toBeInTheDocument()
  })

  it('renders submit button with custom label', () => {
    render(
      <PanelOverlay
        content="# Panel"
        submitLabel="Approve"
        onSubmit={vi.fn()}
        onDismiss={vi.fn()}
      />,
    )

    expect(screen.getByText('Approve')).toBeInTheDocument()
  })

  it('calls onDismiss when dismiss is clicked', () => {
    const onDismiss = vi.fn()
    render(
      <PanelOverlay
        content="# Panel"
        submitLabel="Submit"
        onSubmit={vi.fn()}
        onDismiss={onDismiss}
      />,
    )

    fireEvent.click(screen.getByText('Dismiss'))
    expect(onDismiss).toHaveBeenCalledOnce()
  })

  it('calls onSubmit with serialized selections', () => {
    const onSubmit = vi.fn()
    render(
      <PanelOverlay
        content={SIMPLE_CONTENT}
        submitLabel="Submit"
        onSubmit={onSubmit}
        onDismiss={vi.fn()}
      />,
    )

    // Toggle first checkbox
    const checkboxes = screen.getAllByRole('checkbox')
    fireEvent.click(checkboxes[0])

    // Submit
    fireEvent.click(screen.getByText('Submit'))

    expect(onSubmit).toHaveBeenCalledOnce()
    const result = onSubmit.mock.calls[0][0] as string
    expect(result).toContain('[x] Option A')
    expect(result).toContain('[ ] Option B')
  })

  it('toggles checkbox state on click', () => {
    render(
      <PanelOverlay
        content={SIMPLE_CONTENT}
        submitLabel="Submit"
        onSubmit={vi.fn()}
        onDismiss={vi.fn()}
      />,
    )

    const checkboxes = screen.getAllByRole('checkbox')
    expect(checkboxes[0]).not.toBeChecked()

    fireEvent.click(checkboxes[0])
    expect(checkboxes[0]).toBeChecked()

    fireEvent.click(checkboxes[0])
    expect(checkboxes[0]).not.toBeChecked()
  })
})
