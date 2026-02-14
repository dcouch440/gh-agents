import { useEffect, useMemo, useCallback } from 'react'
import EditNoteOutlined from '@mui/icons-material/EditNoteOutlined'
import { useStore, promptTemplateStore, canvasStore, workflowStore, contextPickerStore } from '@/stores'
import { DESIGN } from '@/constants'
import { Collections } from '@/utils/collections'
import { BrowserPanel } from './BrowserPanel'
import type { PromptTemplate } from '@/types'

const matchesQuery = (template: PromptTemplate, query: string) =>
  template.name.toLowerCase().includes(query.toLowerCase())

const toRow = (template: PromptTemplate) => ({
  primary: template.name,
  secondary: `${template.variables?.length ?? 0} variable(s)`,
})

function PromptsBrowserPanel() {
  const templates = useStore(promptTemplateStore.store, promptTemplateStore.selectAll)
  const loading = useStore(promptTemplateStore.store, promptTemplateStore.selectLoading)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)
  const isPickingActive = useStore(contextPickerStore.store, contextPickerStore.selectActive)

  useEffect(() => {
    void promptTemplateStore.fetchIfStale()
  }, [])

  const firstStepId = useMemo(() => selectedStepIds.values().next().value ?? null, [selectedStepIds])
  const selectedStep = useStore(workflowStore.store, workflowStore.selectStepById(firstStepId))

  const templatesById = useMemo(() => Collections.keyBy(templates, (t) => t.id), [templates])

  const handleAssign = useCallback(
    (templateId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { prompt_template_id: templateId })
    },
    [selectedStep],
  )

  const handlePick = useCallback(
    (templateId: string) => {
      const template = templatesById.get(templateId)
      if (!template) return
      contextPickerStore.pick({
        kind: 'prompt-template',
        id: template.id,
        name: template.name,
        summary: template.description ?? `${template.variables?.length ?? 0} variable(s)`,
        data: template as unknown as Record<string, unknown>,
      })
    },
    [templatesById],
  )

  const isHighlighted = useCallback(
    (template: PromptTemplate) => template.id === selectedStep?.prompt_template_id,
    [selectedStep],
  )

  return (
    <BrowserPanel
      items={templates}
      loading={loading}
      searchPlaceholder="Search templates..."
      emptyIcon={<EditNoteOutlined />}
      emptyLabel="templates"
      barColor={DESIGN.PORT_JSON}
      toRow={toRow}
      matchesQuery={matchesQuery}
      isHighlighted={isHighlighted}
      onItemClick={selectedStep ? handleAssign : null}
      onPickItem={isPickingActive ? handlePick : null}
    />
  )
}

export { PromptsBrowserPanel }
