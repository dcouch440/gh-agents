// ============================================================================
// buildShareableFields — produce ShareableField[] for a node based on archetype
// ============================================================================

import type { ShareableField } from '@/stores/shareStore'
import type { WorkflowStep, DocumentDef, RosterAgent, RoomStepMember } from '@/types/workflow'
import type { Archetype } from './DynamicNode/archetypes'
import { Archetype as ArchetypeEnum, ARCHETYPE_CONFIGS } from './DynamicNode/archetypes'

type BuildShareableFieldsInput = {
  stepId: string
  step: WorkflowStep
  archetype: Archetype
  documentDefs: ReadonlyArray<DocumentDef>
  rosterAgents: ReadonlyArray<RosterAgent>
  roomMembers: ReadonlyArray<RoomStepMember>
}

const buildShareableFields = ({
  stepId,
  step,
  archetype,
  documentDefs,
  rosterAgents,
  roomMembers,
}: BuildShareableFieldsInput): ShareableField[] => {
  const config = ARCHETYPE_CONFIGS[archetype]
  const color = config.color
  const stepName = step.name ?? 'Unnamed'
  const fields: ShareableField[] = []

  // ── General ────────────────────────────────────────────────────────────

  fields.push({
    key: 'name',
    label: 'Name',
    category: 'General',
    kind: 'shared-field',
    color,
    chipKey: 'name',
    entity: {
      kind: 'shared-field',
      id: `${stepId}::name`,
      name: stepName,
      summary: 'Node name',
      data: { fieldType: 'Node Name', value: stepName },
    },
  })

  if (step.description && step.description.trim().length > 0) {
    fields.push({
      key: 'description',
      label: 'Description',
      category: 'General',
      kind: 'shared-field',
      color,
      chipKey: 'description',
      entity: {
        kind: 'shared-field',
        id: `${stepId}::description`,
        name: stepName,
        summary: 'Description',
        data: { fieldType: 'Description', value: step.description },
      },
    })
  }

  if (step.prompt_template && step.prompt_template.trim().length > 0) {
    fields.push({
      key: 'prompt',
      label: 'Prompt',
      category: 'General',
      kind: 'shared-field',
      color,
      chipKey: 'prompt',
      entity: {
        kind: 'shared-field',
        id: `${stepId}::prompt`,
        name: stepName,
        summary: 'Prompt template',
        data: { fieldType: 'Prompt', value: step.prompt_template },
      },
    })
  }

  // ── Documents (DOCUMENTER) ─────────────────────────────────────────────

  if (archetype === ArchetypeEnum.DOCUMENTER) {
    for (const doc of documentDefs) {
      fields.push({
        key: `doc::${doc.id}`,
        label: doc.name,
        category: 'Documents',
        kind: 'document',
        color,
        chipKey: 'doc',
        entity: {
          kind: 'document',
          id: `${stepId}::doc::${doc.id}`,
          name: doc.name,
          summary: `Document from ${stepName}`,
          data: {
            documenterName: stepName,
            description: doc.description,
          },
        },
      })
    }
  }

  // ── Agents (TASK_FORCE) ────────────────────────────────────────────────

  if (archetype === ArchetypeEnum.TASK_FORCE) {
    for (const agent of rosterAgents) {
      fields.push({
        key: `agent::${agent.id}`,
        label: agent.name,
        category: 'Agents',
        kind: 'agent',
        color,
        chipKey: 'agent',
        entity: {
          kind: 'agent',
          id: `${stepId}::agent::${agent.id}`,
          name: agent.name,
          summary: `Roster agent from ${stepName}`,
          data: {
            role_description: agent.role_description,
            capabilities: agent.capabilities.join(', '),
          },
        },
      })
    }
  }

  // ── Members (ROOM) ─────────────────────────────────────────────────────

  if (archetype === ArchetypeEnum.ROOM) {
    for (const member of roomMembers) {
      fields.push({
        key: `member::${member.id}`,
        label: member.name,
        category: 'Members',
        kind: 'shared-field',
        color,
        chipKey: 'member',
        entity: {
          kind: 'shared-field',
          id: `${stepId}::member::${member.id}`,
          name: member.name,
          summary: `Room member from ${stepName}`,
          data: {
            fieldType: 'Room Member',
            value: [
              member.role ? `Role: ${member.role}` : null,
              member.perspective ? `Perspective: ${member.perspective}` : null,
            ]
              .filter(Boolean)
              .join('\n'),
          },
        },
      })
    }
  }

  return fields
}

export { buildShareableFields }
export type { BuildShareableFieldsInput }
