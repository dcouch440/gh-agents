import { describe, it, expect } from 'vitest'
import { api } from './api'

const NAMESPACE_KEYS = [
  'auth', 'agents', 'tools', 'documents', 'sessions',
  'chat', 'config', 'stats', 'agentExecutions', 'outputSchemas', 'promptTemplates',
  'costs', 'results', 'workflows', 'contextResponse', 'modes',
  'rooms', 'roomSessions', 'collections', 'protocols',
] as const

describe('api endpoints', () => {
  it('top-level api object is frozen', () => {
    expect(Object.isFrozen(api)).toBe(true)
  })

  it('all namespace objects are frozen', () => {
    for (const ns of NAMESPACE_KEYS) {
      expect(Object.isFrozen(api[ns]), `api.${ns} should be frozen`).toBe(true)
    }
  })

  it('exposes low-level HTTP methods', () => {
    expect(typeof api.get).toBe('function')
    expect(typeof api.post).toBe('function')
    expect(typeof api.patch).toBe('function')
    expect(typeof api.put).toBe('function')
    expect(typeof api.del).toBe('function')
  })

  it('all namespaces expose only functions', () => {
    for (const ns of NAMESPACE_KEYS) {
      const methods = Object.values(api[ns] as Record<string, unknown>)
      for (const method of methods) {
        expect(typeof method, `api.${ns} contains non-function value`).toBe('function')
      }
    }
  })

  it('all expected namespaces are present', () => {
    for (const ns of NAMESPACE_KEYS) {
      expect(api).toHaveProperty(ns)
    }
  })
})
