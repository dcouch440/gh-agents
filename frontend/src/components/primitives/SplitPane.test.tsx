import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
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
    expect(container.querySelector('.split-pane.custom')).toBeInTheDocument()
  })

  it('handle has onMouseDown', () => {
    const handleMouseDown = vi.fn()
    const { container } = render(
      <SplitPane
        left={<p>L</p>}
        right={<p>R</p>}
        splitPercent={50}
        onMouseDown={handleMouseDown}
      />
    )
    const handle = container.querySelector('.split-pane__handle')
    expect(handle).toBeInTheDocument()
    handle?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))
    expect(handleMouseDown).toHaveBeenCalled()
  })

  it('left panel width style matches splitPercent', () => {
    const { container } = render(
      <SplitPane
        left={<p>L</p>}
        right={<p>R</p>}
        splitPercent={35}
        onMouseDown={vi.fn()}
      />
    )
    const leftPanel = container.querySelector('.split-pane__left') as HTMLElement
    expect(leftPanel.style.width).toBe('35%')
  })
})
