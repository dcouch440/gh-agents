import { describe, it, expect } from 'vitest'
import { buildOccupancyIndex } from './buildOccupancyIndex'

describe('buildOccupancyIndex', () => {
  it('returns empty array for empty input', () => {
    expect(buildOccupancyIndex([], 24)).toEqual([])
  })

  it('builds padded rects from nodes', () => {
    const nodes = [{ id: 'a', rect: { x: 100, y: 200, width: 50, height: 30 } }]
    const result = buildOccupancyIndex(nodes, 10)
    expect(result).toHaveLength(1)
    expect(result[0]!.id).toBe('a')
    expect(result[0]!.rect).toEqual({ x: 100, y: 200, width: 50, height: 30 })
    expect(result[0]!.paddedRect).toEqual({ x: 90, y: 190, width: 70, height: 50 })
  })

  it('handles multiple nodes', () => {
    const nodes = [
      { id: 'a', rect: { x: 0, y: 0, width: 50, height: 50 } },
      { id: 'b', rect: { x: 100, y: 100, width: 50, height: 50 } },
    ]
    const result = buildOccupancyIndex(nodes, 5)
    expect(result).toHaveLength(2)
  })

  it('applies zero padding correctly', () => {
    const nodes = [{ id: 'a', rect: { x: 10, y: 20, width: 30, height: 40 } }]
    const result = buildOccupancyIndex(nodes, 0)
    expect(result[0]!.paddedRect).toEqual(result[0]!.rect)
  })
})
