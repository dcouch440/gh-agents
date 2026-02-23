import { describe, it, expect } from 'vitest'
import { shallowEqual } from './shallowEqual'

describe('shallowEqual', () => {
  it('returns true for identical objects', () => {
    const obj = { a: 1, b: 'hello' }
    expect(shallowEqual(obj, obj)).toBe(true)
  })

  it('returns true for equivalent objects', () => {
    expect(shallowEqual({ a: 1, b: 'hello' }, { a: 1, b: 'hello' })).toBe(true)
  })

  it('returns false for different key counts', () => {
    expect(shallowEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false)
  })

  it('returns false for different values', () => {
    expect(shallowEqual({ a: 1 }, { a: 2 })).toBe(false)
  })

  it('compares arrays by value', () => {
    expect(shallowEqual({ arr: [1, 2, 3] }, { arr: [1, 2, 3] })).toBe(true)
    expect(shallowEqual({ arr: [1, 2] }, { arr: [1, 3] })).toBe(false)
  })

  it('returns false when one is array and other is not', () => {
    expect(shallowEqual({ a: [1] }, { a: 1 })).toBe(false)
  })

  it('uses Object.is for primitives (NaN, -0)', () => {
    expect(shallowEqual({ a: NaN }, { a: NaN })).toBe(true)
    expect(shallowEqual({ a: 0 }, { a: -0 })).toBe(false)
  })

  it('returns true for two empty objects', () => {
    expect(shallowEqual({}, {})).toBe(true)
  })

  it('handles null and undefined values', () => {
    expect(shallowEqual({ a: null }, { a: null })).toBe(true)
    expect(shallowEqual({ a: undefined }, { a: undefined })).toBe(true)
    expect(shallowEqual({ a: null }, { a: undefined })).toBe(false)
  })
})
