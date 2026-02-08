import { useState, useEffect, useMemo, useCallback } from 'react'
import Box from '@mui/material/Box'
import DataObjectOutlined from '@mui/icons-material/DataObjectOutlined'
import { SearchInput, AccentBarRow, EmptyState, LoadingSpinner } from '@/components/primitives'
import { useStore, outputSchemaStore, canvasStore, workflowStore } from '@/stores'
import { DESIGN } from '@/constants'

function SchemasBrowserPanel() {
  const [query, setQuery] = useState('')
  const schemas = useStore(outputSchemaStore.store, outputSchemaStore.selectAll)
  const loading = useStore(outputSchemaStore.store, outputSchemaStore.selectLoading)
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)

  useEffect(() => {
    void outputSchemaStore.fetchIfStale()
  }, [])

  const firstStepId = useMemo(
    () => selectedStepIds.values().next().value ?? null,
    [selectedStepIds],
  )
  const selectedStep = useStore(workflowStore.store, workflowStore.selectStepById(firstStepId))

  const filtered = useMemo(
    () => schemas.filter((s) => s.name.toLowerCase().includes(query.toLowerCase())),
    [schemas, query],
  )

  const handleAssign = useCallback(
    (schemaId: string) => {
      if (!selectedStep) return
      void workflowStore.updateStep(selectedStep.id, { output_schema_id: schemaId })
    },
    [selectedStep],
  )

  const fieldCount = (schema: Record<string, unknown>): number => {
    const props = schema['properties']
    if (props && typeof props === 'object') return Object.keys(props).length
    return Object.keys(schema).length
  }

  return (
    <Box>
      <Box sx={{ px: 1.5, py: 1 }}>
        <SearchInput value={query} onChange={setQuery} placeholder="Search schemas..." />
      </Box>

      {loading ? <LoadingSpinner label="Loading schemas..." /> : null}

      {!loading && filtered.length === 0 ? (
        <EmptyState
          icon={<DataObjectOutlined />}
          message={query ? `No schemas matching "${query}"` : 'No schemas found'}
        />
      ) : null}

      {filtered.map((schema) => (
        <AccentBarRow
          key={schema.id}
          barColor={DESIGN.PORT_ARRAY}
          primary={schema.name}
          secondary={`${fieldCount(schema.schema)} field(s)`}
          highlight={schema.id === selectedStep?.output_schema_id}
          onClick={selectedStep ? () => { handleAssign(schema.id) } : null}
        />
      ))}
    </Box>
  )
}

export { SchemasBrowserPanel }
