import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { SplitPane } from './SplitPane'

describe('SplitPane', () => {
  it('renders left and right content', () => {
    render(
      <SplitPane
        left={<p>Left content</p>}
        right={<p>Right content</p>}
        splitPercent={50}
        onMouseDown={vi.fn()}
      />
    )
    expect(screen.getByText('Left content')).toBeInTheDocument()
    expect(screen.getByText('Right content')).toBeInTheDocument()
  })

  it('applies className when provided', () => {
    const { container } = render(
      <SplitPane
        left={<p>L</p>}
        right={<p>R</p>}
        splitPercent={50}
        onMouseDown={vi.fn()}
        className="custom"
      />
    )
    expect(container.firstChild).toHaveClass('custom')
  })

  it('handle has onMouseDown', () => {
    const handleMouseDown = vi.fn()
    render(
      <SplitPane
        left={<p>L</p>}
        right={<p>R</p>}
        splitPercent={50}
        onMouseDown={handleMouseDown}
      />
    )
    // The handle is the second child (between left and right panels)
    const leftText = screen.getByText('L')
    const handle = leftText.parentElement?.nextElementSibling as HTMLElement
    expect(handle).toBeInTheDocument()
    fireEvent.mouseDown(handle)
    expect(handleMouseDown).toHaveBeenCalled()
  })

  it('left panel width style matches splitPercent', () => {
    const { container, rerender } = render(
      <SplitPane
        left={<p>L</p>}
        right={<p>R</p>}
        splitPercent={35}
        onMouseDown={vi.fn()}
      />
    )
    // The left panel is the first child of the outer Box
    const leftPanel = container.firstChild?.firstChild as HTMLElement
    expect(leftPanel).toBeInTheDocument()
    // MUI sx applies width via a generated CSS class; verify by re-rendering
    // with a different splitPercent and checking the class changes
    const classA = leftPanel.className
    rerender(
      <SplitPane
        left={<p>L</p>}
        right={<p>R</p>}
        splitPercent={60}
        onMouseDown={vi.fn()}
      />
    )
    const classB = leftPanel.className
    expect(classA).not.toBe(classB)
  })
})
