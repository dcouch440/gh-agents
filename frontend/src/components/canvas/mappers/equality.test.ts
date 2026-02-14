import { describe, it, expect } from 'vitest'
import { nodeDataEqual } from './equality'

describe('nodeDataEqual', () => {
  it('returns true for identical objects', () => {
    const a = { kind: 'step', label: 'A', count: 3 }
    expect(nodeDataEqual(a, { ...a })).toBe(true)
  })

  it('returns true for empty objects', () => {
    expect(nodeDataEqual({}, {})).toBe(true)
  })

  it('returns false when key counts differ', () => {
    expect(nodeDataEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false)
  })

  it('returns false when a value differs', () => {
    expect(nodeDataEqual({ a: 1 }, { a: 2 })).toBe(false)
  })

  it('compares arrays element-wise', () => {
    expect(nodeDataEqual({ tags: ['x', 'y'] }, { tags: ['x', 'y'] })).toBe(true)
    expect(nodeDataEqual({ tags: ['x', 'y'] }, { tags: ['x', 'z'] })).toBe(false)
  })

  it('returns false when one value is array and the other is not', () => {
    expect(nodeDataEqual({ tags: ['x'] }, { tags: 'x' })).toBe(false)
  })

  it('returns false for arrays of different length', () => {
    expect(nodeDataEqual({ tags: ['a'] }, { tags: ['a', 'b'] })).toBe(false)
  })

  it('uses Object.is semantics for primitives', () => {
    expect(nodeDataEqual({ v: NaN }, { v: NaN })).toBe(true)
    expect(nodeDataEqual({ v: 0 }, { v: -0 })).toBe(false)
  })

  it('handles null and undefined values', () => {
    expect(nodeDataEqual({ v: null }, { v: null })).toBe(true)
    expect(nodeDataEqual({ v: undefined }, { v: undefined })).toBe(true)
    expect(nodeDataEqual({ v: null }, { v: undefined })).toBe(false)
  })
})
