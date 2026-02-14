import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { PipeEdgePath } from './PipeEdgePath'

const testPath = 'M 0 0 C 50 0, 50 100, 100 100'

const baseProps = {
  edgePath: testPath,
  color: '#3b82f6',
  selected: false,
  isProtocol: true,
  animationDirection: 'normal' as const,
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
    it('renders all 5 layers for protocol edges', () => {
      const container = renderPipe({ isProtocol: true })
      const paths = container.querySelectorAll('path')
      // interaction + glow + body + core + particles = 5
      expect(paths).toHaveLength(5)
    })

    it('renders 3 layers for non-protocol edges (no glow, no particles)', () => {
      const container = renderPipe({ isProtocol: false, selected: false })
      const paths = container.querySelectorAll('path')
      // interaction + body + core = 3
      expect(paths).toHaveLength(3)
    })

    it('renders 5 layers when selected even if not protocol', () => {
      const container = renderPipe({ isProtocol: false, selected: true })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(5)
    })
  })

  describe('glow layer', () => {
    it('renders glow as a wide semi-transparent stroke', () => {
      const container = renderPipe({ isProtocol: true, color: '#3b82f6' })
      const paths = container.querySelectorAll('path')
      // glow is 2nd path (index 1) — no filter, just wide stroke
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
    it('applies the protocol color to body and particle layers', () => {
      const container = renderPipe({ color: '#f85149' })
      const paths = container.querySelectorAll('path')
      // glow (index 1), body (index 2), particles (index 4)
      expect(paths[1]?.getAttribute('stroke')).toBe('#f85149')
      expect(paths[2]?.getAttribute('stroke')).toBe('#f85149')
      expect(paths[4]?.getAttribute('stroke')).toBe('#f85149')
    })

    it('applies a brightened color to the core layer', () => {
      const container = renderPipe({ color: '#000000' })
      const paths = container.querySelectorAll('path')
      // core is index 3; black brightened by 0.4 = #666666
      expect(paths[3]?.getAttribute('stroke')).toBe('#666666')
    })
  })

  describe('animation', () => {
    it('applies forward animation by default', () => {
      const container = renderPipe({ animationDirection: 'normal' })
      const paths = container.querySelectorAll('path')
      const particlePath = paths[4]
      const style = particlePath?.getAttribute('style') ?? ''
      expect(style).toContain('pipeFlow')
      expect(style).not.toContain('pipeFlowReverse')
    })

    it('applies reverse animation when specified', () => {
      const container = renderPipe({ animationDirection: 'reverse' })
      const paths = container.querySelectorAll('path')
      const particlePath = paths[4]
      const style = particlePath?.getAttribute('style') ?? ''
      expect(style).toContain('pipeFlowReverse')
    })
  })
})
