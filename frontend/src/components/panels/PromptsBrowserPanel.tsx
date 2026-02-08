import { useState, useEffect, useMemo, useCallback } from 'react'
import Box from '@mui/material/Box'
import EditNoteOutlined from '@mui/icons-material/EditNoteOutlined'
import { SearchInput, AccentBarRow, EmptyState, LoadingSpinner } from '@/components/primitives'
import { useStore, promptTemplateStore, canvasStore, workflowStore } from '@/stores'
import { DESIGN } from '@/constants'

function PromptsBrowserPanel() {
  const [query, setQuery] = useState('')
  const templates = useStore(promptTemplateStore.store, promptTemplateStore.selectAll)
  const loading = useStore(promptTemplateStore.store, promptTemplateStore.selectLoading)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)

  useEffect(() => {
    void promptTemplateStore.fetchIfStale()
  }, [])

  const selectedStep = useMemo(() => {
    const firstId = selectedStepIds.values().next().value
    if (!firstId) return null
    return steps.find((s) => s.id === firstId) ?? null
  }, [selectedStepIds, steps])

  const filtered = useMemo(
    () => templates.filter((t) => t.name.toLowerCase().includes(query.toLowerCase())),
    [templates, query],
  )

  const handleAssign = useCallback(
    (templateId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { prompt_template_id: templateId })
    },
    [selectedStep],
  )

  return (
    <Box>
      <Box sx={{ px: 1.5, py: 1 }}>
        <SearchInput value={query} onChange={setQuery} placeholder="Search templates..." />
      </Box>

      {loading ? <LoadingSpinner label="Loading templates..." /> : null}

      {!loading && filtered.length === 0 ? (
        <EmptyState
          icon={<EditNoteOutlined />}
          message={query ? `No templates matching "${query}"` : 'No templates found'}
        />
      ) : null}

      {filtered.map((template) => (
        <AccentBarRow
          key={template.id}
          barColor={DESIGN.PORT_JSON}
          primary={template.name}
          secondary={`${template.variables.length} variable(s)`}
          highlight={template.id === selectedStep?.prompt_template_id}
          onClick={selectedStep ? () => { handleAssign(template.id) } : null}
        />
      ))}
    </Box>
  )
}

export { PromptsBrowserPanel }
