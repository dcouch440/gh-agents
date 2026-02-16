import { describe, it, expect } from 'vitest'
import { buildShareableFields } from './buildShareableFields'
import type { WorkflowStep, DocumentDef, RosterAgent, RoomStepMember } from '@/types/workflow'

const makeStep = (overrides: Partial<WorkflowStep> = {}): WorkflowStep => ({
  id: 'step-1',
  workflow_id: 'wf-1',
  name: 'Test Node',
  description: 'A test description',
  execution_mode: 'workforce',
  step_order: 0,
  agent_id: null,
  prompt_template_id: null,
  output_schema_id: null,
  prompt_template: 'Generate docs',
  position_x: 0,
  position_y: 0,
  width: 560,
  height: 500,
  ...overrides,
})

const makeDoc = (id: string, name: string, description: string): DocumentDef => ({
  id,
  step_id: 'step-1',
  name,
  description,
  target_length: 500,
  display_order: 0,
  created_at: '2025-01-01',
})

const makeAgent = (id: string, name: string, role: string): RosterAgent => ({
  id,
  name,
  role_description: role,
  capabilities: ['coding', 'testing'],
  execution_order: 0,
  created_at: '2025-01-01',
})

const makeMember = (id: string, name: string, role: string, perspective: string): RoomStepMember => ({
  id,
  name,
  role,
  perspective,
  display_order: 0,
})

describe('buildShareableFields', () => {
  describe('general fields (all archetypes)', () => {
    it('always includes name', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep({ name: 'My Node', description: '', prompt_template: '' }),
        archetype: 'workforce',
        documentDefs: [],
        rosterAgents: [],
        roomMembers: [],
      })

      const nameField = fields.find((f) => f.key === 'name')
      expect(nameField).toBeDefined()
      expect(nameField!.label).toBe('Name')
      expect(nameField!.category).toBe('General')
      expect(nameField!.chipKey).toBe('name')
      expect(nameField!.entity.data.value).toBe('My Node')
    })

    it('includes description when non-empty', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep({ description: 'Important info' }),
        archetype: 'workforce',
        documentDefs: [],
        rosterAgents: [],
        roomMembers: [],
      })

      const descField = fields.find((f) => f.key === 'description')
      expect(descField).toBeDefined()
      expect(descField!.chipKey).toBe('description')
      expect(descField!.entity.data.value).toBe('Important info')
    })

    it('omits description when empty', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep({ description: '' }),
        archetype: 'workforce',
        documentDefs: [],
        rosterAgents: [],
        roomMembers: [],
      })

      expect(fields.find((f) => f.key === 'description')).toBeUndefined()
    })

    it('includes prompt when non-empty', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep({ prompt_template: 'Do the thing' }),
        archetype: 'workforce',
        documentDefs: [],
        rosterAgents: [],
        roomMembers: [],
      })

      const promptField = fields.find((f) => f.key === 'prompt')
      expect(promptField).toBeDefined()
      expect(promptField!.label).toBe('Content')
      expect(promptField!.chipKey).toBe('content')
      expect(promptField!.entity.data.value).toBe('Do the thing')
    })

    it('omits prompt when empty', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep({ prompt_template: '' }),
        archetype: 'workforce',
        documentDefs: [],
        rosterAgents: [],
        roomMembers: [],
      })

      expect(fields.find((f) => f.key === 'prompt')).toBeUndefined()
    })
  })

  describe('WORKFORCE archetype', () => {
    it('includes document defs', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep(),
        archetype: 'workforce',
        documentDefs: [
          makeDoc('d1', 'README', 'Project readme'),
          makeDoc('d2', 'API Guide', 'API documentation'),
        ],
        rosterAgents: [],
        roomMembers: [],
      })

      const docFields = fields.filter((f) => f.category === 'Documents')
      expect(docFields).toHaveLength(2)
      expect(docFields[0]!.key).toBe('doc::d1')
      expect(docFields[0]!.label).toBe('README')
      expect(docFields[0]!.kind).toBe('document')
      expect(docFields[0]!.chipKey).toBe('doc')
      expect(docFields[1]!.label).toBe('API Guide')
    })

    it('uses workforce color', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep(),
        archetype: 'workforce',
        documentDefs: [makeDoc('d1', 'README', 'docs')],
        rosterAgents: [],
        roomMembers: [],
      })

      expect(fields[0]!.color).toBe('#3b82f6')
    })

    it('includes roster agents', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep(),
        archetype: 'workforce',
        documentDefs: [],
        rosterAgents: [
          makeAgent('a1', 'CodeBot', 'Write code'),
          makeAgent('a2', 'TestBot', 'Write tests'),
        ],
        roomMembers: [],
      })

      const agentFields = fields.filter((f) => f.category === 'Agents')
      expect(agentFields).toHaveLength(2)
      expect(agentFields[0]!.key).toBe('agent::a1')
      expect(agentFields[0]!.label).toBe('CodeBot')
      expect(agentFields[0]!.kind).toBe('agent')
      expect(agentFields[0]!.chipKey).toBe('agent')
    })
  })

  describe('ROOM archetype', () => {
    it('includes room members', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep({ execution_mode: 'room' }),
        archetype: 'room',
        documentDefs: [],
        rosterAgents: [],
        roomMembers: [
          makeMember('m1', 'Alice', 'Facilitator', 'UX focus'),
          makeMember('m2', 'Bob', 'Reviewer', 'Backend focus'),
        ],
      })

      const memberFields = fields.filter((f) => f.category === 'Members')
      expect(memberFields).toHaveLength(2)
      expect(memberFields[0]!.key).toBe('member::m1')
      expect(memberFields[0]!.label).toBe('Alice')
      expect(memberFields[0]!.kind).toBe('shared-field')
      expect(memberFields[0]!.chipKey).toBe('member')
    })
  })

  describe('BLANK archetype', () => {
    it('only includes general fields', () => {
      const fields = buildShareableFields({
        stepId: 'step-1',
        step: makeStep({ execution_mode: 'single', description: 'A blank node', prompt_template: 'Do stuff' }),
        archetype: 'blank',
        documentDefs: [],
        rosterAgents: [],
        roomMembers: [],
      })

      expect(fields).toHaveLength(3) // name, description, prompt
      expect(fields.every((f) => f.category === 'General')).toBe(true)
    })
  })
})
