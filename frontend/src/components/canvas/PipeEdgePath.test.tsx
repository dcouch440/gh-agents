import { describe, it, expect, vi } from 'vitest'
import { render } from '@/test/render'
import { PipeEdgePath } from './PipeEdgePath'

const mockUseCanvasLOD = vi.hoisted(() => vi.fn(() => 'full'))

vi.mock('./useCanvasLOD', () => ({
  useCanvasLOD: mockUseCanvasLOD,
}))

const testPath = 'M 0 0 C 50 0, 50 100, 100 100'

const baseProps = {
  edgePath: testPath,
  color: '#cdc6ba',
  selected: false,
  interactionWidth: 20,
  sourceX: 0,
  sourceY: 0,
  targetX: 100,
  targetY: 100,
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
    it('renders interaction path + dotted path + 2 endpoint dots', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe()
      const paths = container.querySelectorAll('path')
      const circles = container.querySelectorAll('circle')
      expect(paths).toHaveLength(2) // interaction + dotted
      expect(circles).toHaveLength(2) // source + target dots
    })

    it('applies dash array to the connector path', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe()
      const paths = container.querySelectorAll('path')
      const connectorPath = paths[1]
      expect(connectorPath?.getAttribute('stroke-dasharray')).toBe('0.1 20')
    })

    it('applies the color to connector path and endpoint dots', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ color: '#f85149' })
      const paths = container.querySelectorAll('path')
      const circles = container.querySelectorAll('circle')
      expect(paths[1]?.getAttribute('stroke')).toBe('#f85149')
      expect(circles[0]?.getAttribute('fill')).toBe('#f85149')
      expect(circles[1]?.getAttribute('fill')).toBe('#f85149')
    })

    it('places endpoint dots at source and target coordinates', () => {
      mockUseCanvasLOD.mockReturnValue('full')
      const container = renderPipe({ sourceX: 10, sourceY: 20, targetX: 90, targetY: 80 })
      const circles = container.querySelectorAll('circle')
      expect(circles[0]?.getAttribute('cx')).toBe('10')
      expect(circles[0]?.getAttribute('cy')).toBe('20')
      expect(circles[1]?.getAttribute('cx')).toBe('90')
      expect(circles[1]?.getAttribute('cy')).toBe('80')
    })
  })

  describe('minimal detail', () => {
    it('renders only 2 paths (interaction + body), no circles', () => {
      mockUseCanvasLOD.mockReturnValue('minimal')
      const container = renderPipe()
      const paths = container.querySelectorAll('path')
      const circles = container.querySelectorAll('circle')
      expect(paths).toHaveLength(2)
      expect(circles).toHaveLength(0)
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
