import { describe, it, expect } from 'vitest'
import { resolveSubtitle } from './resolveSubtitle'
import { Archetype } from './archetypes'

const defaults = {
  rosterNames: [] as string[],
  documentNames: [] as string[],
  roomMemberNames: [] as string[],
  parentStepName: null as string | null,
}

describe('resolveSubtitle', () => {
  describe('AGENT archetype', () => {
    it('returns parentStepName when provided', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.AGENT, parentStepName: 'Parent Step' })).toBe('Parent Step')
    })

    it('returns null when parentStepName is null', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.AGENT, parentStepName: null })).toBeNull()
    })
  })

  describe('WORKFORCE archetype', () => {
    it('joins roster and document names with middle dot', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.WORKFORCE, rosterNames: ['Alice', 'Bob'], documentNames: ['Report'] })).toBe('Alice \u00b7 Bob \u00b7 Report')
    })

    it('returns null when no roster or document names', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.WORKFORCE })).toBeNull()
    })

    it('handles only roster names', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.WORKFORCE, rosterNames: ['Alice'] })).toBe('Alice')
    })

    it('handles only document names', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.WORKFORCE, documentNames: ['Doc A'] })).toBe('Doc A')
    })
  })

  describe('ROOM archetype', () => {
    it('joins room member names with middle dot', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.ROOM, roomMemberNames: ['Alice', 'Bob'] })).toBe('Alice \u00b7 Bob')
    })

    it('returns null when no room members', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.ROOM })).toBeNull()
    })
  })

  describe('BLANK archetype', () => {
    it('returns null', () => {
      expect(resolveSubtitle({ ...defaults, archetype: Archetype.BLANK })).toBeNull()
    })
  })
})
