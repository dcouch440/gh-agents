import { describe, it, expect } from 'vitest'
import { Archetype, ARCHETYPE_CONFIGS, resolveArchetype } from './archetypes'
import type { ProtocolStepInfo } from '../mappers'

describe('Archetype', () => {
  it('has all expected values', () => {
    expect(Archetype.DOCUMENTER).toBe('documenter')
    expect(Archetype.TASK_FORCE).toBe('task_force')
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

  it('documenter uses orange', () => {
    expect(ARCHETYPE_CONFIGS.documenter.color).toBe('#D4793E')
  })

  it('room uses purple', () => {
    expect(ARCHETYPE_CONFIGS.room.color).toBe('#a78bfa')
  })
})

describe('resolveArchetype', () => {
  const emptyProtocols: ReadonlyMap<string, ProtocolStepInfo> = new Map()

  it('returns DOCUMENTER for execution_mode documenter', () => {
    expect(resolveArchetype({ execution_mode: 'documenter' }, emptyProtocols)).toBe(Archetype.DOCUMENTER)
  })

  it('returns DOCUMENTER when protocol_type is documenter', () => {
    const protocols = new Map<string, ProtocolStepInfo>([
      ['step-1', { protocol_type: 'documenter', name: 'Docs', portNames: [] }],
    ])
    expect(resolveArchetype({ execution_mode: 'single' }, protocols, 'step-1')).toBe(Archetype.DOCUMENTER)
  })

  it('returns ROOM for execution_mode room', () => {
    expect(resolveArchetype({ execution_mode: 'room' }, emptyProtocols)).toBe(Archetype.ROOM)
  })

  it('returns TASK_FORCE for execution_mode agent_designer_input', () => {
    expect(resolveArchetype({ execution_mode: 'agent_designer_input' }, emptyProtocols)).toBe(Archetype.TASK_FORCE)
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
