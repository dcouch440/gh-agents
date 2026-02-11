import { describe, it, expect } from 'vitest'
import { computeHighlightedProtocolIds } from './computeHighlightedProtocolIds'

describe('computeHighlightedProtocolIds', () => {
  it('includes the id of a selected protocol node', () => {
    const nodes = [{ id: 'documenter-1', data: { isProtocol: true } }]
    const result = computeHighlightedProtocolIds(nodes)
    expect(result.has('documenter-1')).toBe(true)
  })

  it('does not include protocolStepId of a non-protocol member node', () => {
    const nodes = [{ id: 'doc-1', data: { protocolStepId: 'documenter-1' } }]
    const result = computeHighlightedProtocolIds(nodes)
    expect(result.has('documenter-1')).toBe(false)
  })

  it('selecting a member does not highlight the whole protocol group', () => {
    const nodes = [{ id: 'doc-1', data: { protocolStepId: 'documenter-1', isProtocol: false } }]
    const result = computeHighlightedProtocolIds(nodes)
    expect(result.size).toBe(0)
  })

  it('selecting both a protocol and a member only adds the protocol id', () => {
    const nodes = [
      { id: 'documenter-1', data: { isProtocol: true } },
      { id: 'doc-1', data: { protocolStepId: 'documenter-1' } },
      { id: 'doc-2', data: { protocolStepId: 'documenter-1' } },
    ]
    const result = computeHighlightedProtocolIds(nodes)
    expect(result.size).toBe(1)
    expect(result.has('documenter-1')).toBe(true)
  })

  it('returns empty set when no nodes are selected', () => {
    const result = computeHighlightedProtocolIds([])
    expect(result.size).toBe(0)
  })

  it('returns empty set for nodes without protocol data', () => {
    const nodes = [{ id: 'step-1', data: { executionMode: 'normal' } }]
    const result = computeHighlightedProtocolIds(nodes)
    expect(result.size).toBe(0)
  })
})
