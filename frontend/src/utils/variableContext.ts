/**
 * Variable context builder for prompt template autocomplete
 *
 * Resolves upstream step output schemas into variable completions
 * for the prompt template editor. Auto-derives variable names from
 * step names when output_variable_name is not set. Generates .$
 * element access syntax for array-type fields and root arrays.
 */

import { extractSchemaFields } from './schemaFields'
import type { SchemaField } from './schemaFields'
import type { WorkflowStep } from '@/types/workflow'
import type { OutputSchema } from '@/types/schema'

type VariableCompletion = {
  label: string
  displayLabel: string
  detail: string
  section: string
}

type StepVariableMapping = {
  stepId: string
  derivedName: string
}

type VariableContext = {
  completions: VariableCompletion[]
  autoNamed: StepVariableMapping[]
}

/**
 * Derive a snake_case variable name from a step name.
 */
const toSnakeCase = (name: string): string =>
  name.trim().toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '')

/**
 * Generate .$ element access completions for a nested array field
 * and its sub-fields.
 */
const addArrayElementCompletions = (
  varName: string,
  field: SchemaField,
  allFields: SchemaField[],
  stepName: string,
  out: VariableCompletion[],
): void => {
  // {var.field.$} — element access
  out.push({
    label: `{${varName}.${field.path}.$}`,
    displayLabel: `${varName}.${field.path}.$`,
    detail: `element \u2014 from ${stepName}`,
    section: stepName,
  })

  // {var.field.$.subfield} for each sub-field of array items
  const prefix = `${field.path}.`
  for (const sub of allFields) {
    if (sub.path.startsWith(prefix)) {
      const subPath = sub.path.slice(prefix.length)
      out.push({
        label: `{${varName}.${field.path}.$.${subPath}}`,
        displayLabel: `${varName}.${field.path}.$.${subPath}`,
        detail: `${sub.type} \u2014 from ${stepName}`,
        section: stepName,
      })
    }
  }
}

/**
 * Build the list of variable completions available for a step's prompt template.
 *
 * Auto-derives variable names from step names when output_variable_name is
 * not set. Generates .$ element access syntax for arrays dynamically from
 * schema types.
 */
const buildVariableCompletions = (
  upstreamIds: readonly string[],
  stepsById: ReadonlyMap<string, WorkflowStep>,
  schemas: ReadonlyMap<string, OutputSchema>,
  _currentStep: WorkflowStep | null,
): VariableContext => {
  const completions: VariableCompletion[] = []
  const autoNamed: StepVariableMapping[] = []

  for (const upId of upstreamIds) {
    const upStep = stepsById.get(upId)
    if (!upStep) continue

    const stepName = upStep.name ?? upStep.execution_mode

    // Use existing name or auto-derive from step name
    let varName = upStep.output_variable_name
    if (!varName) {
      varName = toSnakeCase(stepName)
      if (!varName) continue
      autoNamed.push({ stepId: upStep.id, derivedName: varName })
    }

    // Root variable
    const schema = upStep.output_schema_id ? schemas.get(upStep.output_schema_id) : undefined
    const rootType = schema
      ? (typeof schema.schema.type === 'string' ? schema.schema.type : 'object')
      : 'any'
    const isRootArray = rootType === 'array'

    completions.push({
      label: `{${varName}}`,
      displayLabel: varName,
      detail: `${rootType} \u2014 from ${stepName}`,
      section: stepName,
    })

    // Field paths when schema is available
    if (schema) {
      const fields = extractSchemaFields(schema.schema)

      if (isRootArray) {
        // Root-level array: show {var.$} and {var.$.field}
        completions.push({
          label: `{${varName}.$}`,
          displayLabel: `${varName}.$`,
          detail: `element \u2014 from ${stepName}`,
          section: stepName,
        })
        for (const field of fields) {
          completions.push({
            label: `{${varName}.$.${field.path}}`,
            displayLabel: `${varName}.$.${field.path}`,
            detail: `${field.type} \u2014 from ${stepName}`,
            section: stepName,
          })
        }
      } else {
        for (const field of fields) {
          completions.push({
            label: `{${varName}.${field.path}}`,
            displayLabel: `${varName}.${field.path}`,
            detail: `${field.type} \u2014 from ${stepName}`,
            section: stepName,
          })

          // Array element access for nested arrays
          if (field.type === 'array') {
            addArrayElementCompletions(varName, field, fields, stepName, completions)
          }
        }
      }
    }
  }

  return { completions, autoNamed }
}

export { buildVariableCompletions, toSnakeCase }
export type { VariableCompletion, VariableContext, StepVariableMapping }
