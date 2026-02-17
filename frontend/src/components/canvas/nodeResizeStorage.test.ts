import { describe, it, expect } from 'vitest'
import { isVirtualNode } from './nodeResizeStorage'

describe('isVirtualNode', () => {
  it('returns true for doc-artifact nodes', () => {
    expect(isVirtualNode('doc-artifact-abc-123')).toBe(true)
  })

  it('returns true for notes nodes', () => {
    expect(isVirtualNode('notes-abc-123')).toBe(true)
  })

  it('returns true for agent-artifact nodes', () => {
    expect(isVirtualNode('agent-artifact-abc-123')).toBe(true)
  })

  it('returns false for regular step IDs', () => {
    expect(isVirtualNode('abc-123-def-456')).toBe(false)
  })

  it('returns false for empty string', () => {
    expect(isVirtualNode('')).toBe(false)
  })
})
