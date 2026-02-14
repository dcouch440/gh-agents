import { useEffect, useMemo, useCallback } from 'react'
import DataObjectOutlined from '@mui/icons-material/DataObjectOutlined'
import { useStore, outputSchemaStore, canvasStore, workflowStore } from '@/stores'
import { DESIGN } from '@/constants'
import { BrowserPanel } from './BrowserPanel'
import type { OutputSchema } from '@/types'

const matchesQuery = (schema: OutputSchema, query: string) => schema.name.toLowerCase().includes(query.toLowerCase())

const fieldCount = (schema: Record<string, unknown>): number => {
  const props = schema['properties']
  if (props && typeof props === 'object') return Object.keys(props).length
  return Object.keys(schema).length
}

const toRow = (schema: OutputSchema) => ({
  primary: schema.name,
  secondary: `${fieldCount(schema.schema)} field(s)`,
})

function SchemasBrowserPanel() {
  const schemas = useStore(outputSchemaStore.store, outputSchemaStore.selectAll)
  const loading = useStore(outputSchemaStore.store, outputSchemaStore.selectLoading)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)

  useEffect(() => {
    void outputSchemaStore.fetchIfStale()
  }, [])

  const firstStepId = useMemo(() => selectedStepIds.values().next().value ?? null, [selectedStepIds])
  const selectedStep = useStore(workflowStore.store, workflowStore.selectStepById(firstStepId))

  const handleAssign = useCallback(
    (schemaId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { output_schema_id: schemaId })
    },
    [selectedStep],
  )

  const isHighlighted = useCallback((schema: OutputSchema) => schema.id === selectedStep?.output_schema_id, [selectedStep])

  return (
    <BrowserPanel
      items={schemas}
      loading={loading}
      searchPlaceholder="Search schemas..."
      emptyIcon={<DataObjectOutlined />}
      emptyLabel="schemas"
      barColor={DESIGN.PORT_ARRAY}
      toRow={toRow}
      matchesQuery={matchesQuery}
      isHighlighted={isHighlighted}
      onItemClick={selectedStep ? handleAssign : null}
    />
  )
}

export { SchemasBrowserPanel }
