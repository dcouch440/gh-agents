import { describe, it, expect } from 'vitest'
import { parseDocArtifactId, findParentStepForDef, buildProtocolsByStep, buildDocArtifactShareFields } from './canvasContextMenuUtils'
import type { WorkflowStep, DocumentDef } from '@/types/workflow'
import type { StepProtocolLink } from '@/stores'

describe('parseDocArtifactId', () => {
  it('extracts defId from doc-artifact- prefixed string', () => {
    expect(parseDocArtifactId('doc-artifact-abc-123')).toBe('abc-123')
  })

  it('returns null for non-prefixed string', () => {
    expect(parseDocArtifactId('step-123')).toBeNull()
  })

  it('returns empty string for prefix-only', () => {
    expect(parseDocArtifactId('doc-artifact-')).toBe('')
  })
})

describe('findParentStepForDef', () => {
  const documentDefsByStep: Record<string, ReadonlyArray<DocumentDef>> = {
    'step-1': [
      { id: 'def-a', step_id: 'step-1', name: 'Doc A', description: '', target_length: 100, display_order: 0, created_at: '', document_id: null, agent_roster_entry_id: null },
      { id: 'def-b', step_id: 'step-1', name: 'Doc B', description: '', target_length: 100, display_order: 1, created_at: '', document_id: null, agent_roster_entry_id: null },
    ],
    'step-2': [
      { id: 'def-c', step_id: 'step-2', name: 'Doc C', description: '', target_length: 100, display_order: 0, created_at: '', document_id: null, agent_roster_entry_id: null },
    ],
  }

  it('finds parent step for matching def', () => {
    expect(findParentStepForDef(documentDefsByStep, 'def-b')).toBe('step-1')
  })

  it('finds parent step in second entry', () => {
    expect(findParentStepForDef(documentDefsByStep, 'def-c')).toBe('step-2')
  })

  it('returns null for unknown def', () => {
    expect(findParentStepForDef(documentDefsByStep, 'def-unknown')).toBeNull()
  })

  it('returns null for empty map', () => {
    expect(findParentStepForDef({}, 'def-a')).toBeNull()
  })
})

describe('buildProtocolsByStep', () => {
  it('builds lookup from step protocol links', () => {
    const stepProtocols: Readonly<Record<string, StepProtocolLink>> = {
      'step-1': { protocolType: 'workforce', protocolName: 'WF', portNames: [] },
      'step-2': { protocolType: 'room', protocolName: 'RM', portNames: [] },
    }
    const result = buildProtocolsByStep(stepProtocols)

    expect(result.get('step-1')).toEqual({ protocol_type: 'workforce' })
    expect(result.get('step-2')).toEqual({ protocol_type: 'room' })
    expect(result.size).toBe(2)
  })

  it('returns empty map for empty input', () => {
    expect(buildProtocolsByStep({}).size).toBe(0)
  })
})

describe('buildDocArtifactShareFields', () => {
  const parentStep = {
    id: 'step-1',
    workflow_id: 'wf-1',
    name: 'My Workforce',
    description: '',
    execution_mode: 'workforce',
    step_order: 0,
    agent_id: null,
    prompt_template_id: null,
    output_schema_id: null,
    prompt_template: '',
    position_x: 0,
    position_y: 0,
    width: 560,
    height: 500,
  } as WorkflowStep

  const targetDef: DocumentDef = {
    id: 'def-1',
    step_id: 'step-1',
    name: 'Research Report',
    description: 'A detailed report',
    target_length: 500,
    display_order: 0,
    created_at: '',
    document_id: null,
    agent_roster_entry_id: null,
  }

  const emptyProtocols: ReadonlyMap<string, { protocol_type: string }> = new Map()

  it('produces a single ShareableField for the document', () => {
    const fields = buildDocArtifactShareFields('def-1', parentStep, 'step-1', targetDef, emptyProtocols)

    expect(fields).toHaveLength(1)
    expect(fields[0]!.key).toBe('doc::def-1')
    expect(fields[0]!.label).toBe('Research Report')
    expect(fields[0]!.category).toBe('Documents')
    expect(fields[0]!.kind).toBe('document')
    expect(fields[0]!.entity.name).toBe('Research Report')
    expect(fields[0]!.entity.id).toBe('step-1::doc::def-1')
    expect(fields[0]!.entity.summary).toBe('Document from My Workforce')
  })

  it('uses "Unnamed" when parent step has no name', () => {
    const unnamed = { ...parentStep, name: null } as WorkflowStep
    const fields = buildDocArtifactShareFields('def-1', unnamed, 'step-1', targetDef, emptyProtocols)

    expect(fields[0]!.entity.summary).toBe('Document from Unnamed')
  })
})
