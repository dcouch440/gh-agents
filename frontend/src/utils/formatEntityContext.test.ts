import { describe, it, expect } from 'vitest'
import { formatEntityContext } from './formatEntityContext'
import type { PickableEntity } from '@/stores/contextMentionStore'

const makeEntity = (kind: PickableEntity['kind'], name: string, data: Record<string, unknown>): PickableEntity => ({
  kind,
  id: 'test-id',
  name,
  summary: '',
  data,
})

describe('formatEntityContext', () => {
  describe('formatSharedField', () => {
    it('formats as fieldType: value', () => {
      const entity = makeEntity('shared-field', 'MyNode', { fieldType: 'Name', value: 'MyNode' })
      expect(formatEntityContext(entity)).toBe('Name: MyNode')
    })

    it('falls back to entity name when value is empty', () => {
      const entity = makeEntity('shared-field', 'MyNode', { fieldType: 'Name', value: '' })
      expect(formatEntityContext(entity)).toBe('Name: MyNode')
    })

    it('formats description field', () => {
      const entity = makeEntity('shared-field', 'StepName', {
        fieldType: 'Description',
        value: 'Generates API docs from source code',
      })
      expect(formatEntityContext(entity)).toBe('Description: Generates API docs from source code')
    })

    it('uses Field as default fieldType', () => {
      const entity = makeEntity('shared-field', 'Something', { value: 'hello' })
      expect(formatEntityContext(entity)).toBe('Field: hello')
    })
  })

  describe('formatDocument', () => {
    it('formats with description', () => {
      const entity = makeEntity('document', 'README', {
        description: 'Project readme file',
        parentStepName: 'DocWriter',
      })
      expect(formatEntityContext(entity)).toBe('Document "README": Project readme file')
    })

    it('formats without description', () => {
      const entity = makeEntity('document', 'README', { parentStepName: 'DocWriter' })
      expect(formatEntityContext(entity)).toBe('Document "README"')
    })

    it('formats with empty description', () => {
      const entity = makeEntity('document', 'README', { description: '', parentStepName: 'DocWriter' })
      expect(formatEntityContext(entity)).toBe('Document "README"')
    })
  })

  describe('formatAgent', () => {
    it('formats with role description', () => {
      const entity = makeEntity('agent', 'CodeBot', { role_description: 'Write code and review PRs' })
      expect(formatEntityContext(entity)).toBe('Agent "CodeBot": Write code and review PRs')
    })

    it('formats without role description', () => {
      const entity = makeEntity('agent', 'CodeBot', {})
      expect(formatEntityContext(entity)).toBe('Agent "CodeBot"')
    })
  })

  describe('formatContextNode (unchanged)', () => {
    it('preserves existing bracket format', () => {
      const entity = makeEntity('context-node', 'My Context', { content: 'some content' })
      expect(formatEntityContext(entity)).toBe('[Context: Context Node] My Context\nContent:\nsome content')
    })
  })
})
