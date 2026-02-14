import type { PickableEntity, PickableEntityKind } from '@/stores/contextMentionStore'

type EntityFormatter = (entity: PickableEntity) => string

const str = (value: unknown): string | null => (typeof value === 'string' && value.length > 0 ? value : null)

const formatAgent: EntityFormatter = (entity) => {
  const d = entity.data
  const role = str(d.role_description)
  return role ? `Agent "${entity.name}": ${role}` : `Agent "${entity.name}"`
}

const formatPromptTemplate: EntityFormatter = (entity) => {
  const d = entity.data
  return [
    `[Context: Prompt Template] ${entity.name}`,
    str(d.description) ? `Description: ${str(d.description)}` : null,
    str(d.template) ? `Template:\n${str(d.template)}` : null,
    Array.isArray(d.variables) && d.variables.length > 0 ? `Variables: ${(d.variables as string[]).join(', ')}` : null,
  ]
    .filter(Boolean)
    .join('\n')
}

const formatOutputSchema: EntityFormatter = (entity) => {
  const d = entity.data
  return [
    `[Context: Output Schema] ${entity.name}`,
    d.schema ? `Schema:\n${JSON.stringify(d.schema, null, 2)}` : null,
  ]
    .filter(Boolean)
    .join('\n')
}

const formatWorkflowStep: EntityFormatter = (entity) => {
  const d = entity.data
  return [
    `[Context: Workflow Step] ${entity.name}`,
    str(d.archetype) ? `Archetype: ${str(d.archetype)}` : null,
    str(d.description) ? `Description: ${str(d.description)}` : null,
  ]
    .filter(Boolean)
    .join('\n')
}

const formatDocument: EntityFormatter = (entity) => {
  const d = entity.data
  const desc = str(d.description)
  return desc ? `Document "${entity.name}": ${desc}` : `Document "${entity.name}"`
}

const formatContextNode: EntityFormatter = (entity) => {
  const d = entity.data
  return [
    `[Context: Context Node] ${entity.name}`,
    str(d.content) ? `Content:\n${str(d.content)}` : null,
  ]
    .filter(Boolean)
    .join('\n')
}

const formatSharedField: EntityFormatter = (entity) => {
  const d = entity.data
  const fieldType = str(d.fieldType) ?? 'Field'
  const value = str(d.value)
  return value ? `${fieldType}: ${value}` : `${fieldType}: ${entity.name}`
}

const FORMATTERS: Record<PickableEntityKind, EntityFormatter> = {
  'agent': formatAgent,
  'prompt-template': formatPromptTemplate,
  'output-schema': formatOutputSchema,
  'workflow-step': formatWorkflowStep,
  'document': formatDocument,
  'context-node': formatContextNode,
  'shared-field': formatSharedField,
}

const formatEntityContext = (entity: PickableEntity): string => {
  const formatter = FORMATTERS[entity.kind]
  return formatter(entity)
}

export { formatEntityContext, FORMATTERS }
export type { EntityFormatter }
