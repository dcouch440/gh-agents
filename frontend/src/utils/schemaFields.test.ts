import { describe, it, expect } from 'vitest'
import { extractSchemaFields } from './schemaFields'

describe('extractSchemaFields', () => {
  it('extracts flat string/number/boolean properties', () => {
    const schema = {
      type: 'object',
      properties: {
        name: { type: 'string', description: 'The name' },
        count: { type: 'number' },
        active: { type: 'boolean' },
      },
    }
    const fields = extractSchemaFields(schema)

    expect(fields).toEqual([
      { path: 'name', type: 'string', description: 'The name' },
      { path: 'count', type: 'number', description: null },
      { path: 'active', type: 'boolean', description: null },
    ])
  })

  it('extracts nested object properties with dot-paths', () => {
    const schema = {
      type: 'object',
      properties: {
        metadata: {
          type: 'object',
          properties: {
            author: { type: 'string' },
            version: { type: 'number' },
          },
        },
      },
    }
    const fields = extractSchemaFields(schema)

    expect(fields).toEqual([
      { path: 'metadata', type: 'object', description: null },
      { path: 'metadata.author', type: 'string', description: null },
      { path: 'metadata.version', type: 'number', description: null },
    ])
  })

  it('extracts array item fields', () => {
    const schema = {
      type: 'object',
      properties: {
        items: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              title: { type: 'string' },
              score: { type: 'number' },
            },
          },
        },
      },
    }
    const fields = extractSchemaFields(schema)

    expect(fields).toEqual([
      { path: 'items', type: 'array', description: null },
      { path: 'items.title', type: 'string', description: null },
      { path: 'items.score', type: 'number', description: null },
    ])
  })

  it('respects maxDepth limit', () => {
    const schema = {
      type: 'object',
      properties: {
        level1: {
          type: 'object',
          properties: {
            level2: {
              type: 'object',
              properties: {
                level3: {
                  type: 'object',
                  properties: {
                    deep: { type: 'string' },
                  },
                },
              },
            },
          },
        },
      },
    }

    // maxDepth=2 should stop before level3's children
    const fields = extractSchemaFields(schema, 2)
    const paths = fields.map((f) => f.path)

    expect(paths).toContain('level1')
    expect(paths).toContain('level1.level2')
    expect(paths).toContain('level1.level2.level3')
    expect(paths).not.toContain('level1.level2.level3.deep')
  })

  it('returns empty array for schema with no properties', () => {
    expect(extractSchemaFields({})).toEqual([])
    expect(extractSchemaFields({ type: 'object' })).toEqual([])
  })

  it('handles properties with missing type gracefully', () => {
    const schema = {
      type: 'object',
      properties: {
        mystery: { description: 'no type field' },
      },
    }
    const fields = extractSchemaFields(schema)

    expect(fields).toEqual([
      { path: 'mystery', type: 'unknown', description: 'no type field' },
    ])
  })

  it('preserves descriptions through nesting', () => {
    const schema = {
      type: 'object',
      properties: {
        result: {
          type: 'object',
          description: 'The result object',
          properties: {
            summary: { type: 'string', description: 'Brief summary' },
          },
        },
      },
    }
    const fields = extractSchemaFields(schema)

    expect(fields).toEqual([
      { path: 'result', type: 'object', description: 'The result object' },
      { path: 'result.summary', type: 'string', description: 'Brief summary' },
    ])
  })
})
