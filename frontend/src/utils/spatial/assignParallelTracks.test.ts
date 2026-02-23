import { describe, it, expect } from 'vitest'
import { assignParallelTracks } from './assignParallelTracks'

describe('assignParallelTracks', () => {
  it('returns empty array for zero count', () => {
    expect(assignParallelTracks(0, 8)).toEqual([])
  })

  it('returns [0] for a single track', () => {
    expect(assignParallelTracks(1, 8)).toEqual([0])
  })

  it('centers two tracks around zero', () => {
    expect(assignParallelTracks(2, 10)).toEqual([-5, 5])
  })

  it('centers three tracks around zero', () => {
    expect(assignParallelTracks(3, 8)).toEqual([-8, 0, 8])
  })

  it('centers four tracks around zero', () => {
    expect(assignParallelTracks(4, 6)).toEqual([-9, -3, 3, 9])
  })

  it('returns empty for negative count', () => {
    expect(assignParallelTracks(-1, 8)).toEqual([])
  })
})
