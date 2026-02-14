import type { PickableEntity, PickableEntityKind } from '@/stores/contextMentionStore'

type EntityFormatter = (entity: PickableEntity) => string

const str = (value: unknown): string | null => (typeof value === 'string' && value.length > 0 ? value : null)

const formatAgent: EntityFormatter = (entity) => {
  const d = entity.data
  return [
    `[Context: Agent] ${entity.name}`,
    str(d.model_id) ? `Model: ${str(d.model_id)}` : null,
    str(d.system_prompt) ? `System Prompt:\n${str(d.system_prompt)}` : null,
    str(d.status) ? `Status: ${str(d.status)}` : null,
  ]
    .filter(Boolean)
    .join('\n')
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
  return [
    `[Context: Document] ${entity.name}`,
    str(d.documenterName) ? `Source: ${str(d.documenterName)}` : null,
    str(d.content) ? `Content:\n${str(d.content)}` : null,
  ]
    .filter(Boolean)
    .join('\n')
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
  return [
    `[Context: ${fieldType}] ${entity.name}`,
    str(d.value) ? `${str(d.value)}` : null,
  ]
    .filter(Boolean)
    .join('\n')
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
