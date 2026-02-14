import { useEffect, useMemo, useCallback } from 'react'
import EditNoteOutlined from '@mui/icons-material/EditNoteOutlined'
import { useStore, promptTemplateStore, canvasStore, workflowStore } from '@/stores'
import { DESIGN } from '@/constants'
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

  useEffect(() => {
    void promptTemplateStore.fetchIfStale()
  }, [])

  const firstStepId = useMemo(() => selectedStepIds.values().next().value ?? null, [selectedStepIds])
  const selectedStep = useStore(workflowStore.store, workflowStore.selectStepById(firstStepId))

  const handleAssign = useCallback(
    (templateId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { prompt_template_id: templateId })
    },
    [selectedStep],
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
    />
  )
}

export { PromptsBrowserPanel }
