import { shallow } from './shallow'

describe('shallow', () => {
  it('returns true for identical references', () => {
    const obj = { a: 1, b: 2 }
    expect(shallow(obj, obj)).toBe(true)
  })

  it('returns true for equal primitives', () => {
    expect(shallow(42, 42)).toBe(true)
    expect(shallow('hello', 'hello')).toBe(true)
    expect(shallow(true, true)).toBe(true)
    expect(shallow(null, null)).toBe(true)
    expect(shallow(undefined, undefined)).toBe(true)
  })

  it('returns false for different primitives', () => {
    expect(shallow(1, 2)).toBe(false)
    expect(shallow('a', 'b')).toBe(false)
    expect(shallow(true, false)).toBe(false)
  })

  it('returns true for shallow-equal objects', () => {
    expect(shallow({ a: 1, b: 'two' }, { a: 1, b: 'two' })).toBe(true)
  })

  it('returns false for different values', () => {
    expect(shallow({ a: 1 }, { a: 2 })).toBe(false)
  })

  it('returns false for different key counts', () => {
    expect(shallow({ a: 1 }, { a: 1, b: 2 })).toBe(false)
    expect(shallow({ a: 1, b: 2 }, { a: 1 })).toBe(false)
  })

  it('returns false when comparing object to null', () => {
    expect(shallow({ a: 1 }, null)).toBe(false)
    expect(shallow(null, { a: 1 })).toBe(false)
  })

  it('returns false when comparing object to primitive', () => {
    expect(shallow({ a: 1 } as unknown, 42 as unknown)).toBe(false)
    expect(shallow(42 as unknown, { a: 1 } as unknown)).toBe(false)
  })

  it('handles arrays (treated as objects with index keys)', () => {
    expect(shallow([1, 2, 3], [1, 2, 3])).toBe(true)
    expect(shallow([1, 2], [1, 3])).toBe(false)
    expect(shallow([1, 2], [1, 2, 3])).toBe(false)
  })

  it('returns false for nested object differences', () => {
    const inner = { x: 1 }
    expect(shallow({ a: inner }, { a: inner })).toBe(true)
    expect(shallow({ a: { x: 1 } }, { a: { x: 1 } })).toBe(false) // different references
  })
})
