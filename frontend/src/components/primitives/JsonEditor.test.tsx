import {describe, it, expect, vi} from 'vitest'
import {render} from '@testing-library/react'
import {JsonEditor} from './JsonEditor'
import {formatJson, validateJsonObject} from '@/utils/json'

describe('JsonEditor', () => {
  it('renders without crashing', () => {
    const onChange = vi.fn()
    const initialValue = '{"name": "test"}'

    const {container} = render(<JsonEditor value={initialValue} onChange={onChange} />)

    expect(container).toBeDefined()
  })

  it('renders with placeholder', () => {
    const onChange = vi.fn()
    const placeholder = 'Enter JSON here'

    const {container} = render(<JsonEditor value="" onChange={onChange} placeholder={placeholder} />)

    expect(container).toBeDefined()
  })

  it('renders with readOnly mode', () => {
    const onChange = vi.fn()

    const {container} = render(<JsonEditor value='{"test": true}' onChange={onChange} readOnly />)

    expect(container).toBeDefined()
  })
})

describe('formatJson', () => {
  it('formats valid JSON correctly', () => {
    const input = '{"name":"test","value":123}'
    const expected = '{\n  "name": "test",\n  "value": 123\n}'

    expect(formatJson(input)).toBe(expected)
  })

  it('returns original text for invalid JSON', () => {
    const invalid = '{invalid json'

    expect(formatJson(invalid)).toBe(invalid)
  })

  it('handles nested objects', () => {
    const input = '{"a":{"b":{"c":1}}}'
    const result = formatJson(input)

    expect(result).toContain('  "a": {')
    expect(result).toContain('    "b": {')
    expect(result).toContain('      "c": 1')
  })
})

describe('validateJsonObject', () => {
  it('validates correct JSON object', () => {
    const input = '{"name": "test", "value": 123}'
    const result = validateJsonObject(input)

    expect(result.valid).toBe(true)
    expect(result.parsed).toEqual({name: 'test', value: 123})
    expect(result.error).toBeUndefined()
  })

  it('catches syntax errors', () => {
    const invalid = '{invalid json'
    const result = validateJsonObject(invalid)

    expect(result.valid).toBe(false)
    expect(result.error).toBeDefined()
    expect(result.parsed).toBeUndefined()
  })

  it('rejects arrays', () => {
    const array = '["a", "b", "c"]'
    const result = validateJsonObject(array)

    expect(result.valid).toBe(false)
    expect(result.error).toBe('JSON must be an object')
  })

  it('rejects null', () => {
    const nullValue = 'null'
    const result = validateJsonObject(nullValue)

    expect(result.valid).toBe(false)
    expect(result.error).toBe('JSON must be an object')
  })

  it('rejects primitives', () => {
    const string = '"test"'
    const result = validateJsonObject(string)

    expect(result.valid).toBe(false)
    expect(result.error).toBe('JSON must be an object')
  })

  it('accepts empty object', () => {
    const empty = '{}'
    const result = validateJsonObject(empty)

    expect(result.valid).toBe(true)
    expect(result.parsed).toEqual({})
  })

  it('accepts nested objects', () => {
    const nested = '{"a": {"b": {"c": 1}}}'
    const result = validateJsonObject(nested)

    expect(result.valid).toBe(true)
    expect(result.parsed).toEqual({a: {b: {c: 1}}})
  })
})
