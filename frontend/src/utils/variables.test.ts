import { describe, it, expect } from 'vitest'
import { extractVariables, resolveVariables, validateVariableData } from './variables'

describe('extractVariables', () => {
  it('extracts root variables from simple placeholders', () => {
    const template = 'Hello {user}'
    expect(extractVariables(template)).toEqual(['user'])
  })

  it('extracts multiple root variables', () => {
    const template = 'Hello {user} and {admin}'
    expect(extractVariables(template)).toEqual(['admin', 'user']) // Sorted
  })

  it('deduplicates root variables from nested paths', () => {
    const template = '{output.items} and {output.total} and {output.summary}'
    expect(extractVariables(template)).toEqual(['output'])
  })

  it('extracts mixed root and nested variables', () => {
    const template = '{user.name} and {admin.email} and {count}'
    expect(extractVariables(template)).toEqual(['admin', 'count', 'user'])
  })

  it('handles no variables', () => {
    const template = 'No variables here'
    expect(extractVariables(template)).toEqual([])
  })

  it('ignores invalid variable syntax', () => {
    const template = '{123invalid} {_valid} {also-invalid}'
    expect(extractVariables(template)).toEqual(['_valid'])
  })

  it('handles array index paths', () => {
    const template = '{items.0.name} and {items.1.value}'
    expect(extractVariables(template)).toEqual(['items'])
  })

  it('handles deep nesting', () => {
    const template = '{data.nested.deep.very.deep.value}'
    expect(extractVariables(template)).toEqual(['data'])
  })

  it('handles special for-each syntax with $', () => {
    const template = '{items.$.name}'
    expect(extractVariables(template)).toEqual(['items'])
  })

  it('handles empty string', () => {
    expect(extractVariables('')).toEqual([])
  })

  it('handles variables at start and end', () => {
    const template = '{start} middle {end}'
    expect(extractVariables(template)).toEqual(['end', 'start'])
  })

  it('handles multiple occurrences of same variable', () => {
    const template = '{user} and {user} and {user}'
    expect(extractVariables(template)).toEqual(['user'])
  })
})

describe('resolveVariables', () => {
  it('resolves simple string variable', () => {
    const template = 'Hello {name}'
    const mockData = { name: '"Alice"' }
    expect(resolveVariables(template, mockData)).toBe('Hello Alice')
  })

  it('resolves simple number variable', () => {
    const template = 'Count: {count}'
    const mockData = { count: '42' }
    expect(resolveVariables(template, mockData)).toBe('Count: 42')
  })

  it('resolves simple boolean variable', () => {
    const template = 'Active: {active}'
    const mockData = { active: 'true' }
    expect(resolveVariables(template, mockData)).toBe('Active: true')
  })

  it('resolves object variable as JSON', () => {
    const template = 'Data: {data}'
    const mockData = { data: '{"key":"value"}' }
    expect(resolveVariables(template, mockData)).toBe('Data: {"key":"value"}')
  })

  it('resolves nested object field', () => {
    const template = 'Total: {output.total}'
    const mockData = { output: '{"total":42}' }
    expect(resolveVariables(template, mockData)).toBe('Total: 42')
  })

  it('resolves dot-path with multiple levels', () => {
    const template = 'Value: {data.nested.value}'
    const mockData = { data: '{"nested":{"value":"result"}}' }
    expect(resolveVariables(template, mockData)).toBe('Value: result')
  })

  it('resolves array by index', () => {
    const template = 'First: {items.0}'
    const mockData = { items: '["a","b","c"]' }
    expect(resolveVariables(template, mockData)).toBe('First: a')
  })

  it('resolves array element field', () => {
    const template = 'Name: {features.0.name}'
    const mockData = { features: '[{"name":"foo"},{"name":"bar"}]' }
    expect(resolveVariables(template, mockData)).toBe('Name: foo')
  })

  it('resolves deep array nesting', () => {
    const template = 'Value: {data.items.2.metadata.title}'
    const mockData = {
      data: '{"items":[{},{},{"metadata":{"title":"test"}}]}',
    }
    expect(resolveVariables(template, mockData)).toBe('Value: test')
  })

  it('resolves multiple variables in one template', () => {
    const template = 'Hello {user.name}, your score is {score}'
    const mockData = {
      user: '{"name":"Alice"}',
      score: '95',
    }
    expect(resolveVariables(template, mockData)).toBe('Hello Alice, your score is 95')
  })

  it('leaves unresolved variables unchanged', () => {
    const template = 'Hello {missing}'
    expect(resolveVariables(template, {})).toBe('Hello {missing}')
  })

  it('leaves variables with invalid JSON unchanged', () => {
    const template = 'Value: {data.value}'
    const mockData = { data: 'not json' }
    expect(resolveVariables(template, mockData)).toBe('Value: {data.value}')
  })

  it('leaves variables with missing nested path unchanged', () => {
    const template = 'Value: {output.missing.path}'
    const mockData = { output: '{"other":"field"}' }
    expect(resolveVariables(template, mockData)).toBe('Value: {output.missing.path}')
  })

  it('leaves variables with out-of-bounds array index unchanged', () => {
    const template = 'Value: {items.10}'
    const mockData = { items: '["a","b"]' }
    expect(resolveVariables(template, mockData)).toBe('Value: {items.10}')
  })

  it('handles empty mock data', () => {
    const template = 'Hello {user}'
    expect(resolveVariables(template, {})).toBe('Hello {user}')
  })

  it('handles template with no variables', () => {
    const template = 'No variables here'
    const mockData = { data: '{"value":42}' }
    expect(resolveVariables(template, mockData)).toBe('No variables here')
  })

  it('resolves mixed resolved and unresolved variables', () => {
    const template = 'Resolved: {good}, Unresolved: {bad}'
    const mockData = { good: '"works"' }
    expect(resolveVariables(template, mockData)).toBe('Resolved: works, Unresolved: {bad}')
  })

  it('handles variables with empty string value', () => {
    const template = 'Value: {data}'
    const mockData = { data: '""' }
    expect(resolveVariables(template, mockData)).toBe('Value: ')
  })

  it('handles variables with null value in JSON', () => {
    const template = 'Value: {data.nullField}'
    const mockData = { data: '{"nullField":null}' }
    expect(resolveVariables(template, mockData)).toBe('Value: {data.nullField}')
  })

  it('stringifies array results', () => {
    const template = 'Items: {data.items}'
    const mockData = { data: '{"items":["a","b","c"]}' }
    expect(resolveVariables(template, mockData)).toBe('Items: ["a","b","c"]')
  })

  it('stringifies nested object results', () => {
    const template = 'Metadata: {data.metadata}'
    const mockData = { data: '{"metadata":{"key":"value","count":5}}' }
    expect(resolveVariables(template, mockData)).toBe('Metadata: {"key":"value","count":5}')
  })
})

describe('validateVariableData', () => {
  it('validates valid JSON object', () => {
    const result = validateVariableData('{"key":"value"}')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('validates valid JSON array', () => {
    const result = validateVariableData('[1,2,3]')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('validates valid JSON string', () => {
    const result = validateVariableData('"text"')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('validates valid JSON number', () => {
    const result = validateVariableData('42')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('validates valid JSON boolean', () => {
    const result = validateVariableData('true')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('validates valid JSON null', () => {
    const result = validateVariableData('null')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('accepts empty string as valid', () => {
    const result = validateVariableData('')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('accepts whitespace-only string as valid', () => {
    const result = validateVariableData('   \n  \t  ')
    expect(result.valid).toBe(true)
    expect(result.error).toBeUndefined()
  })

  it('rejects invalid JSON syntax', () => {
    const result = validateVariableData('{invalid}')
    expect(result.valid).toBe(false)
    expect(result.error).toBeDefined()
  })

  it('rejects unclosed braces', () => {
    const result = validateVariableData('{"key":"value"')
    expect(result.valid).toBe(false)
    expect(result.error).toBeDefined()
  })

  it('rejects unclosed brackets', () => {
    const result = validateVariableData('[1,2,3')
    expect(result.valid).toBe(false)
    expect(result.error).toBeDefined()
  })

  it('rejects trailing comma', () => {
    const result = validateVariableData('{"key":"value",}')
    expect(result.valid).toBe(false)
    expect(result.error).toBeDefined()
  })

  it('rejects single quotes instead of double quotes', () => {
    const result = validateVariableData("{'key':'value'}")
    expect(result.valid).toBe(false)
    expect(result.error).toBeDefined()
  })

  it('rejects unquoted keys', () => {
    const result = validateVariableData('{key:"value"}')
    expect(result.valid).toBe(false)
    expect(result.error).toBeDefined()
  })

  it('provides error message for invalid JSON', () => {
    const result = validateVariableData('not json')
    expect(result.valid).toBe(false)
    expect(result.error).toContain('JSON')
  })
})
