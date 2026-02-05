import { Box } from '@mui/material'
import { DataTable, EmptyState, Button, type Column } from '@/components/primitives'
import type { RouterMode } from '@/types'

type RouterModesListProps = {
  modes: RouterMode[]
  onEditMode: (mode: RouterMode) => void
  onDeleteMode: (mode: RouterMode) => void
  onManageTools: (mode: RouterMode) => void
}

function RouterModesList({ modes, onEditMode, onDeleteMode, onManageTools }: RouterModesListProps) {
  if (modes.length === 0) {
    return <EmptyState message="No modes configured. Add a mode to get started." />
  }

  const columns: Column<RouterMode>[] = [
    {
      key: 'display_name',
      header: 'Name',
      render: (mode) => mode.display_name,
    },
    {
      key: 'mode_key',
      header: 'Key',
      render: (mode) => mode.mode_key,
    },
    {
      key: 'temperature',
      header: 'Temp',
      render: (mode) => mode.temperature.toFixed(1),
    },
    {
      key: 'max_tokens',
      header: 'Max Tokens',
      render: (mode) => mode.max_tokens.toLocaleString(),
    },
    {
      key: 'actions',
      header: 'Actions',
      render: (mode) => (
        <Box sx={{ display: 'flex', gap: 0.5 }}>
          <Button variant="secondary" size="small" onClick={() => onManageTools(mode)}>
            Tools
          </Button>
          <Button variant="secondary" size="small" onClick={() => onEditMode(mode)}>
            Edit
          </Button>
          <Button variant="danger" size="small" onClick={() => onDeleteMode(mode)}>
            Delete
          </Button>
        </Box>
      ),
    },
  ]

  return <DataTable columns={columns} rows={modes} rowKey={(m) => m.id} />
}

export { RouterModesList }
export type { RouterModesListProps }
