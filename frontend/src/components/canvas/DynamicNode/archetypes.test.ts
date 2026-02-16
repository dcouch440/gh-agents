import { describe, it, expect } from 'vitest'
import { Archetype, ARCHETYPE_CONFIGS, resolveArchetype } from './archetypes'
import type { ProtocolStepInfo } from '../mappers'

describe('Archetype', () => {
  it('has all expected values', () => {
    expect(Archetype.WORKFORCE).toBe('workforce')
    expect(Archetype.ROOM).toBe('room')
    expect(Archetype.BLANK).toBe('blank')
  })
})

describe('ARCHETYPE_CONFIGS', () => {
  it('has a config for every archetype', () => {
    const archetypes = Object.values(Archetype)
    for (const a of archetypes) {
      expect(ARCHETYPE_CONFIGS[a]).toBeDefined()
      expect(ARCHETYPE_CONFIGS[a].label).toBeTruthy()
      expect(ARCHETYPE_CONFIGS[a].color).toMatch(/^#[0-9a-fA-F]{6}$/)
      expect(ARCHETYPE_CONFIGS[a].executionMode).toBeTruthy()
    }
  })

  it('workforce uses blue', () => {
    expect(ARCHETYPE_CONFIGS.workforce.color).toBe('#3b82f6')
  })

  it('room uses purple', () => {
    expect(ARCHETYPE_CONFIGS.room.color).toBe('#a78bfa')
  })
})

describe('resolveArchetype', () => {
  const emptyProtocols: ReadonlyMap<string, ProtocolStepInfo> = new Map()

  it('returns WORKFORCE for execution_mode workforce', () => {
    expect(resolveArchetype({ execution_mode: 'workforce' }, emptyProtocols)).toBe(Archetype.WORKFORCE)
  })

  it('returns ROOM for execution_mode room', () => {
    expect(resolveArchetype({ execution_mode: 'room' }, emptyProtocols)).toBe(Archetype.ROOM)
  })

  it('returns BLANK for execution_mode single', () => {
    expect(resolveArchetype({ execution_mode: 'single' }, emptyProtocols)).toBe(Archetype.BLANK)
  })

  it('returns BLANK for execution_mode for_each', () => {
    expect(resolveArchetype({ execution_mode: 'for_each' }, emptyProtocols)).toBe(Archetype.BLANK)
  })

  it('returns BLANK for unknown execution modes', () => {
    expect(resolveArchetype({ execution_mode: 'unknown' }, emptyProtocols)).toBe(Archetype.BLANK)
  })
})
