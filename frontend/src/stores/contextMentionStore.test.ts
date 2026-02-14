import { describe, it, expect, beforeEach } from 'vitest'
import { contextMentionStore } from './contextMentionStore'
import type { PickableEntity } from './contextMentionStore'

const makeEntity = (id: string, name: string, kind: PickableEntity['kind'] = 'context-node'): PickableEntity => ({
  kind,
  id,
  name,
  summary: `${kind}: ${name}`,
  data: { content: `content of ${name}` },
})

describe('contextMentionStore', () => {
  beforeEach(() => {
    contextMentionStore.reset()
  })

  describe('addMention', () => {
    it('adds a mention for a step', () => {
      const entity = makeEntity('e1', 'Context A')
      contextMentionStore.addMention('step1', entity, '#10b981')

      const mentions = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      expect(mentions).toHaveLength(1)
      expect(mentions[0]!.entityId).toBe('e1')
      expect(mentions[0]!.label).toBe('Context A')
      expect(mentions[0]!.color).toBe('#10b981')
      expect(mentions[0]!.entity).toBe(entity)
      expect(mentions[0]!.id).toBeTruthy()
    })

    it('deduplicates by entityId', () => {
      const entity = makeEntity('e1', 'Context A')
      contextMentionStore.addMention('step1', entity, '#10b981')
      contextMentionStore.addMention('step1', entity, '#10b981')

      const mentions = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      expect(mentions).toHaveLength(1)
    })

    it('allows same entity on different steps', () => {
      const entity = makeEntity('e1', 'Context A')
      contextMentionStore.addMention('step1', entity, '#10b981')
      contextMentionStore.addMention('step2', entity, '#10b981')

      const m1 = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      const m2 = contextMentionStore.selectMentions('step2')(contextMentionStore.store.getState())
      expect(m1).toHaveLength(1)
      expect(m2).toHaveLength(1)
    })

    it('adds multiple different entities to the same step', () => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'A'), '#10b981')
      contextMentionStore.addMention('step1', makeEntity('e2', 'B'), '#3b82f6')

      const mentions = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      expect(mentions).toHaveLength(2)
      expect(mentions[0]!.entityId).toBe('e1')
      expect(mentions[1]!.entityId).toBe('e2')
    })
  })

  describe('removeMention', () => {
    it('removes a mention by token id', () => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'A'), '#10b981')
      contextMentionStore.addMention('step1', makeEntity('e2', 'B'), '#3b82f6')

      const before = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      contextMentionStore.removeMention('step1', before[0]!.id)

      const after = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      expect(after).toHaveLength(1)
      expect(after[0]!.entityId).toBe('e2')
    })

    it('no-ops when step has no mentions', () => {
      contextMentionStore.removeMention('step1', 'nonexistent')
      const mentions = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      expect(mentions).toHaveLength(0)
    })
  })

  describe('removeByEntityId', () => {
    it('removes a mention by entity id', () => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'A'), '#10b981')
      contextMentionStore.addMention('step1', makeEntity('e2', 'B'), '#3b82f6')

      contextMentionStore.removeByEntityId('step1', 'e1')

      const after = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      expect(after).toHaveLength(1)
      expect(after[0]!.entityId).toBe('e2')
    })

    it('no-ops for unknown entity id', () => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'A'), '#10b981')
      contextMentionStore.removeByEntityId('step1', 'unknown')

      const after = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      expect(after).toHaveLength(1)
    })
  })

  describe('clearStep', () => {
    it('clears all mentions for a step', () => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'A'), '#10b981')
      contextMentionStore.addMention('step1', makeEntity('e2', 'B'), '#3b82f6')
      contextMentionStore.addMention('step2', makeEntity('e3', 'C'), '#a78bfa')

      contextMentionStore.clearStep('step1')

      const m1 = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      const m2 = contextMentionStore.selectMentions('step2')(contextMentionStore.store.getState())
      expect(m1).toHaveLength(0)
      expect(m2).toHaveLength(1)
    })
  })

  describe('reset', () => {
    it('clears all state', () => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'A'), '#10b981')
      contextMentionStore.addMention('step2', makeEntity('e2', 'B'), '#3b82f6')

      contextMentionStore.reset()

      const m1 = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
      const m2 = contextMentionStore.selectMentions('step2')(contextMentionStore.store.getState())
      expect(m1).toHaveLength(0)
      expect(m2).toHaveLength(0)
    })
  })

  describe('selectEntityIds', () => {
    it('returns a set of entity ids for a step', () => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'A'), '#10b981')
      contextMentionStore.addMention('step1', makeEntity('e2', 'B'), '#3b82f6')

      const ids = contextMentionStore.selectEntityIds('step1')(contextMentionStore.store.getState())
      expect(ids).toEqual(new Set(['e1', 'e2']))
    })

    it('returns empty set for unknown step', () => {
      const ids = contextMentionStore.selectEntityIds('unknown')(contextMentionStore.store.getState())
      expect(ids.size).toBe(0)
    })
  })

  describe('selectMentions', () => {
    it('returns stable empty array for unknown step', () => {
      const a = contextMentionStore.selectMentions('unknown')(contextMentionStore.store.getState())
      const b = contextMentionStore.selectMentions('unknown')(contextMentionStore.store.getState())
      expect(a).toBe(b)
    })
  })
})
