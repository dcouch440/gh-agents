/**
 * Variable context builder for prompt template autocomplete
 *
 * Resolves upstream step output schemas into variable completions
 * for the prompt template editor.
 */

import { extractSchemaFields } from './schemaFields'
import type { WorkflowStep } from '@/types/workflow'
import type { OutputSchema } from '@/types/schema'

type VariableCompletion = {
  label: string
  displayLabel: string
  detail: string
  section: string
}

/**
 * Build the list of variable completions available for a step's prompt template.
 *
 * Receives pre-computed upstream step IDs (from the store's adjacency map),
 * then resolves their output schemas to extract field paths. Only steps with
 * both `output_variable_name` and `output_schema_id` produce completions.
 */
const buildVariableCompletions = (
  upstreamIds: readonly string[],
  stepsById: ReadonlyMap<string, WorkflowStep>,
  schemas: ReadonlyMap<string, OutputSchema>,
): VariableCompletion[] => {
  const completions: VariableCompletion[] = []

  for (const upId of upstreamIds) {
    const upStep = stepsById.get(upId)
    if (!upStep) continue

    const varName = upStep.output_variable_name
    if (!varName) continue

    const schemaId = upStep.output_schema_id
    if (!schemaId) continue

    const schema = schemas.get(schemaId)
    if (!schema) continue

    const stepName = upStep.name ?? upStep.execution_mode
    const fields = extractSchemaFields(schema.schema)

    // Add the root variable itself
    completions.push({
      label: `{${varName}}`,
      displayLabel: varName,
      detail: `object \u2014 from ${stepName}`,
      section: stepName,
    })

    // Add each field path
    for (const field of fields) {
      completions.push({
        label: `{${varName}.${field.path}}`,
        displayLabel: `${varName}.${field.path}`,
        detail: `${field.type} \u2014 from ${stepName}`,
        section: stepName,
      })
    }
  }

  return completions
}

export { buildVariableCompletions }
export type { VariableCompletion }
