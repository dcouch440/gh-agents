import { describe, it, expect, vi } from 'vitest'
import { render } from '@testing-library/react'
import { PipeEdgePath } from './PipeEdgePath'

const mockUseCanvasLOD = vi.hoisted(() => vi.fn(() => 'full'))

vi.mock('./useCanvasLOD', () => ({
  useCanvasLOD: mockUseCanvasLOD,
}))

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
  describe('full detail', () => {
    it('renders 4 layers for protocol edges (interaction + glow + body + core)', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ isProtocol: true })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(4)
    })

    it('renders 3 layers for non-protocol edges (interaction + body + core, no glow)', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ isProtocol: false, selected: false })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(3)
    })

    it('renders 4 layers when selected even if not protocol', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ isProtocol: false, selected: true })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(4)
    })

    it('renders glow as a wide semi-transparent stroke', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ isProtocol: true, color: '#3b82f6' })
      const paths = container.querySelectorAll('path')
      const glowPath = paths[1]
      expect(glowPath?.getAttribute('stroke')).toBe('#3b82f6')
      expect(glowPath?.getAttribute('filter')).toBeNull()
    })

    it('applies the color to glow and body layers', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ color: '#f85149' })
      const paths = container.querySelectorAll('path')
      expect(paths[1]?.getAttribute('stroke')).toBe('#f85149')
      expect(paths[2]?.getAttribute('stroke')).toBe('#f85149')
    })

    it('applies a brightened color to the core layer', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ color: '#000000' })
      const paths = container.querySelectorAll('path')
      expect(paths[3]?.getAttribute('stroke')).toBe('#666666')
    })
  })

  describe('minimal detail', () => {
    it('renders only 2 paths (interaction + body)', () => {
      mockUseCanvasLOD.mockReturnValue('minimal')
      const container = renderPipe({ isProtocol: true })
      const paths = container.querySelectorAll('path')
      expect(paths).toHaveLength(2)
    })

    it('keeps the interaction hit area', () => {
      mockUseCanvasLOD.mockReturnValue('minimal')
      const container = renderPipe()
      const interactionPath = container.querySelector('.react-flow__edge-interaction')
      expect(interactionPath).toBeInTheDocument()
    })

    it('renders a single body path with the edge color', () => {
      mockUseCanvasLOD.mockReturnValue('minimal')
      const container = renderPipe({ color: '#f85149' })
      const paths = container.querySelectorAll('path')
      expect(paths[1]?.getAttribute('stroke')).toBe('#f85149')
    })
  })

  describe('interaction layer', () => {
    it('has the interaction class', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe()
      const interactionPath = container.querySelector('.react-flow__edge-interaction')
      expect(interactionPath).toBeInTheDocument()
    })

    it('uses the specified interaction width', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ interactionWidth: 30 })
      const interactionPath = container.querySelector('.react-flow__edge-interaction')
      expect(interactionPath?.getAttribute('stroke-width')).toBe('30')
    })
  })
})
