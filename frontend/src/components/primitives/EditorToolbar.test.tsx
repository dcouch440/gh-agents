import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EditorToolbar } from './EditorToolbar'

describe('EditorToolbar', () => {
  it('renders children', () => {
    render(<EditorToolbar><span>child content</span></EditorToolbar>)
    expect(screen.getByText('child content')).toBeInTheDocument()
  })

  it('applies className', () => {
    const { container } = render(<EditorToolbar className="extra">content</EditorToolbar>)
    expect(container.firstChild).toHaveClass('extra')
  })
})
