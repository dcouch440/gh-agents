import { describe, it, expect } from 'vitest'
import { parseWaypoints } from './parseWaypoints'

describe('parseWaypoints', () => {
  it('returns empty array for empty string', () => {
    expect(parseWaypoints('')).toEqual([])
  })

  it('parses a single M command', () => {
    expect(parseWaypoints('M 10 20')).toEqual([{ x: 10, y: 20 }])
  })

  it('parses M followed by L commands', () => {
    expect(parseWaypoints('M 0 0 L 100 0 L 100 50')).toEqual([
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 100, y: 50 },
    ])
  })

  it('handles negative coordinates', () => {
    expect(parseWaypoints('M -10 -20 L 30 -40')).toEqual([
      { x: -10, y: -20 },
      { x: 30, y: -40 },
    ])
  })

  it('handles decimal coordinates', () => {
    expect(parseWaypoints('M 10.5 20.75 L 30.25 40.1')).toEqual([
      { x: 10.5, y: 20.75 },
      { x: 30.25, y: 40.1 },
    ])
  })

  it('ignores non-M/L commands', () => {
    expect(parseWaypoints('M 0 0 Q 50 50 100 100 L 200 200')).toEqual([
      { x: 0, y: 0 },
      { x: 200, y: 200 },
    ])
  })
})
