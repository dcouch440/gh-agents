import { useCallback } from 'react'
import { workflowStore } from '@/stores'
import type { WorkflowStep, PromptTemplate } from '@/types'

type UseStepFieldHandlersArgs = {
  stepId: string
  templatesMap: ReadonlyMap<string, PromptTemplate>
}

type UseStepFieldHandlersResult = {
  handleFieldChange: (field: 'name' | 'prompt_template' | 'system_prompt_suffix', value: string) => void
  handleAgentChange: (agentId: string | null) => void
  handleTemplateChange: (templateId: string | null) => void
  handleSchemaChange: (schemaId: string | null) => void
  handleCopyVariable: (label: string) => void
}

const useStepFieldHandlers = ({ stepId, templatesMap }: UseStepFieldHandlersArgs): UseStepFieldHandlersResult => {
  const handleFieldChange = useCallback(
    (field: 'name' | 'prompt_template' | 'system_prompt_suffix', value: string) => {
      const storeValue = field === 'prompt_template' ? value : value || null
      workflowStore.patchStepLocal(stepId, { [field]: storeValue } as Partial<WorkflowStep>)
    },
    [stepId],
  )

  const handleAgentChange = useCallback(
    (agentId: string | null) => {
      if (agentId !== null) {
        workflowStore.patchStepLocal(stepId, { agent_id: agentId })
      }
    },
    [stepId],
  )

  const handleTemplateChange = useCallback(
    (templateId: string | null) => {
      const tpl = templateId ? templatesMap.get(templateId) : undefined
      workflowStore.patchStepLocal(stepId, {
        prompt_template_id: templateId,
        prompt_template: tpl?.template ?? '',
      })
    },
    [stepId, templatesMap],
  )

  const handleSchemaChange = useCallback(
    (schemaId: string | null) => {
      workflowStore.patchStepLocal(stepId, { output_schema_id: schemaId })
    },
    [stepId],
  )

  const handleCopyVariable = useCallback((label: string) => {
    void navigator.clipboard.writeText(label)
  }, [])

  return { handleFieldChange, handleAgentChange, handleTemplateChange, handleSchemaChange, handleCopyVariable }
}

export { useStepFieldHandlers }
export type { UseStepFieldHandlersArgs, UseStepFieldHandlersResult }
