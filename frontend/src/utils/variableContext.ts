/**
 * Variable context builder for prompt template autocomplete
 *
 * Analyzes the workflow graph to determine what upstream variables
 * are available for a given step, based on edges, output_variable_name,
 * and output schemas.
 */

import { extractSchemaFields } from './schemaFields'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
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
 * Walks incoming edges to find upstream steps, then resolves their output schemas
 * to extract field paths. Only steps with both `output_variable_name` and
 * `output_schema_id` produce completions.
 */
const buildVariableCompletions = (
  currentStepId: string,
  stepsById: ReadonlyMap<string, WorkflowStep>,
  edges: ReadonlyArray<WorkflowStepEdge>,
  schemas: ReadonlyMap<string, OutputSchema>,
): VariableCompletion[] => {
  const completions: VariableCompletion[] = []
  console.log(edges)
  // Find upstream step IDs from edges pointing to this step
  const upstreamIds = new Set(
    edges
      .filter((e) => e.to_step_id === currentStepId)
      .map((e) => e.from_step_id),
  )

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

  console.log(completions)

  return completions
}

export { buildVariableCompletions }
export type { VariableCompletion }
