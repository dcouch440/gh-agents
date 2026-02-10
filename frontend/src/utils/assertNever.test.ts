import { describe, it, expect } from 'vitest'
import { assertNever } from './assertNever'

describe('assertNever', () => {
  it('throws with the value in the error message', () => {
    const value = { type: 'unknown', id: '123' } as never
    expect(() => assertNever(value)).toThrow('Unhandled discriminated union member')
    expect(() => assertNever(value)).toThrow('"unknown"')
  })

  it('throws for string values', () => {
    const value = 'unexpected' as never
    expect(() => assertNever(value)).toThrow('"unexpected"')
  })

  it('throws for numeric values', () => {
    const value = 42 as never
    expect(() => assertNever(value)).toThrow('42')
  })
})
