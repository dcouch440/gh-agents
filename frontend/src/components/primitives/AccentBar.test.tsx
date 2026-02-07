import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { AccentBar } from './AccentBar'

describe('AccentBar', () => {
  it('renders a bar element', () => {
    const { container } = render(<AccentBar color="#3b82f6" />)
    const bar = container.firstChild as HTMLElement
    expect(bar).toBeInTheDocument()
    expect(bar.tagName).toBe('DIV')
  })

  it('renders with MUI Box class', () => {
    const { container } = render(<AccentBar color="#ff0000" />)
    const bar = container.firstChild as HTMLElement
    expect(bar.className).toContain('MuiBox-root')
  })
})
