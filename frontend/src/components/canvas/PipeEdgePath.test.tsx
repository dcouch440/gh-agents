import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { PipeEdgePath } from './PipeEdgePath'

const testPath = 'M 0 0 C 50 0, 50 100, 100 100'

const baseProps = {
  edgePath: testPath,
  color: '#3b82f6',
  selected: false,
  isProtocol: true,
  interactionWidth: 20,
}

const renderPipe = (overrides: Partial<typeof baseProps> = {}) => {
  const { container } = render(
    <svg>
      <PipeEdgePath {...baseProps} {...overrides} />
    </svg>,
  )
  return container
}

describe('PipeEdgePath', () => {
  describe('layer rendering', () => {
    it('renders 4 layers for protocol edges (interaction + glow + body + core)', () => {
      const container = renderPipe({ isProtocol: true })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(4)
    })

    it('renders 3 layers for non-protocol edges (interaction + body + core, no glow)', () => {
      const container = renderPipe({ isProtocol: false, selected: false })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(3)
    })

    it('renders 4 layers when selected even if not protocol', () => {
      const container = renderPipe({ isProtocol: false, selected: true })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(4)
    })
  })

  describe('glow layer', () => {
    it('renders glow as a wide semi-transparent stroke', () => {
      const container = renderPipe({ isProtocol: true, color: '#3b82f6' })
      const paths = container.querySelectorAll('path')
      // glow is 2nd path (index 1)
      const glowPath = paths[1]
      expect(glowPath?.getAttribute('stroke')).toBe('#3b82f6')
      expect(glowPath?.getAttribute('filter')).toBeNull()
    })
  })

  describe('interaction layer', () => {
    it('has the interaction class', () => {
      const container = renderPipe()
      const interactionPath = container.querySelector('.react-flow__edge-interaction')
      expect(interactionPath).toBeInTheDocument()
    })

    it('uses the specified interaction width', () => {
      const container = renderPipe({ interactionWidth: 30 })
      const interactionPath = container.querySelector('.react-flow__edge-interaction')
      expect(interactionPath?.getAttribute('stroke-width')).toBe('30')
    })
  })

  describe('color application', () => {
    it('applies the color to glow and body layers', () => {
      const container = renderPipe({ color: '#f85149' })
      const paths = container.querySelectorAll('path')
      // glow (index 1), body (index 2)
      expect(paths[1]?.getAttribute('stroke')).toBe('#f85149')
      expect(paths[2]?.getAttribute('stroke')).toBe('#f85149')
    })

    it('applies a brightened color to the core layer', () => {
      const container = renderPipe({ color: '#000000' })
      const paths = container.querySelectorAll('path')
      // core is index 3; black brightened by 0.4 = #666666
      expect(paths[3]?.getAttribute('stroke')).toBe('#666666')
    })
  })
})
