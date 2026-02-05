import { useState } from 'react'
import {
  Box,
  Button,
  Alert,
  Typography,
  CircularProgress,
  IconButton,
  Chip,
} from '@mui/material'
import EditIcon from '@mui/icons-material/Edit'
import DeleteIcon from '@mui/icons-material/Delete'
import BuildIcon from '@mui/icons-material/Build'
import { PageHeader } from '@/components/primitives'
import { DataTable } from '@/components/primitives/DataTable'
import type { Column } from '@/components/primitives/DataTable'
import { ModeFormDialog } from './ModeFormDialog'
import { ModeToolSelector } from './ModeToolSelector'
import { useRouterModes } from '@/hooks/useRouterModes'
import { useRouterModeMutations } from '@/hooks/useRouterModeMutations'
import { useTools } from '@/hooks/useTools'
import type { RouterMode } from '@/types'

const DEFAULT_ROUTER_ID = 'default-router'

function RouterModesTab() {
  const { modes, loading, error, reload } = useRouterModes(DEFAULT_ROUTER_ID)
  const {
    deleteMode,
    updating,
    deleting,
    loadModeTools,
    saveModeTools,
    loadingTools,
    savingTools,
    toolsError,
  } = useRouterModeMutations()
  const { tools } = useTools()

  const [formDialogOpen, setFormDialogOpen] = useState(false)
  const [toolSelectorOpen, setToolSelectorOpen] = useState(false)
  const [selectedMode, setSelectedMode] = useState<RouterMode | null>(null)
  const [deleteError, setDeleteError] = useState<string | null>(null)

  const handleCreate = () => {
    setSelectedMode(null)
    setFormDialogOpen(true)
  }

  const handleEdit = (mode: RouterMode) => {
    setSelectedMode(mode)
    setFormDialogOpen(true)
  }

  const handleFormSave = async () => {
    await reload()
    setFormDialogOpen(false)
    setSelectedMode(null)
  }

  const handleFormClose = () => {
    setFormDialogOpen(false)
    setSelectedMode(null)
  }

  const handleToolsClick = (mode: RouterMode) => {
    setSelectedMode(mode)
    setToolSelectorOpen(true)
  }

  const handleToolsClose = () => {
    setToolSelectorOpen(false)
    setSelectedMode(null)
  }

  const handleDelete = async (mode: RouterMode) => {
    if (!confirm(`Delete mode "${mode.display_name}"? This cannot be undone.`)) {
      return
    }

    setDeleteError(null)
    try {
      await deleteMode(mode.id)
      await reload()
    } catch (err) {
      setDeleteError(err instanceof Error ? err.message : 'Failed to delete mode')
    }
  }

  const columns: Column<RouterMode>[] = [
    {
      key: 'mode_key',
      header: 'Mode Key',
      sortable: true,
      render: (row) => (
        <Typography variant="body2" component="code" sx={{ fontFamily: 'monospace' }}>
          {row.mode_key}
        </Typography>
      ),
    },
    {
      key: 'display_name',
      header: 'Display Name',
      sortable: true,
      render: (row) => (
        <Typography variant="body2" sx={{ fontWeight: 500 }}>
          {row.display_name}
        </Typography>
      ),
    },
    {
      key: 'description',
      header: 'Description',
      sortable: false,
      render: (row) => (
        <Typography
          variant="body2"
          color="text.secondary"
          sx={{
            maxWidth: 300,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {row.description || '—'}
        </Typography>
      ),
    },
    {
      key: 'settings',
      header: 'Settings',
      sortable: false,
      render: (row) => (
        <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
          {row.append_to_agent_system_prompt && (
            <Chip label="Append Prompt" size="small" variant="outlined" />
          )}
          {row.append_to_agent_tools && (
            <Chip label="Append Tools" size="small" variant="outlined" />
          )}
          <Chip
            label={`T: ${row.temperature}`}
            size="small"
            variant="outlined"
          />
        </Box>
      ),
    },
    {
      key: 'actions',
      header: 'Actions',
      sortable: false,
      render: (row) => (
        <Box sx={{ display: 'flex', gap: 0.5 }}>
          <IconButton
            size="small"
            onClick={() => handleEdit(row)}
            disabled={updating || deleting}
            aria-label="Edit mode"
          >
            <EditIcon fontSize="small" />
          </IconButton>
          <IconButton
            size="small"
            onClick={() => handleToolsClick(row)}
            disabled={updating || deleting}
            aria-label="Manage tools"
          >
            <BuildIcon fontSize="small" />
          </IconButton>
          <IconButton
            size="small"
            onClick={() => {
              void handleDelete(row)
            }}
            disabled={updating || deleting}
            aria-label="Delete mode"
            color="error"
          >
            <DeleteIcon fontSize="small" />
          </IconButton>
        </Box>
      ),
    },
  ]

  return (
    <Box>
      <PageHeader title="Router Modes">
        <Button variant="contained" onClick={handleCreate} disabled={loading}>
          Create Mode
        </Button>
      </PageHeader>

      {error && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}

      {deleteError && (
        <Alert
          severity="error"
          sx={{ mb: 2 }}
          onClose={() => setDeleteError(null)}
        >
          {deleteError}
        </Alert>
      )}

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
          <CircularProgress />
        </Box>
      ) : modes.length === 0 ? (
        <Box sx={{ textAlign: 'center', py: 4 }}>
          <Typography variant="body1" color="text.secondary">
            No router modes configured. Create your first mode to get started.
          </Typography>
        </Box>
      ) : (
        <DataTable
          columns={columns}
          rows={modes}
          rowKey={(row) => row.id}
          sortColumn={null}
          sortDirection="asc"
          onSort={null}
        />
      )}

      <ModeFormDialog
        open={formDialogOpen}
        onClose={handleFormClose}
        onSave={() => {
          void handleFormSave()
        }}
        mode={selectedMode}
        routerId={DEFAULT_ROUTER_ID}
      />

      {selectedMode && (
        <ModeToolSelector
          open={toolSelectorOpen}
          onClose={handleToolsClose}
          mode={selectedMode}
          allTools={tools}
          loadModeTools={loadModeTools}
          saveModeTools={saveModeTools}
          loadingTools={loadingTools}
          savingTools={savingTools}
          toolsError={toolsError}
        />
      )}
    </Box>
  )
}

export { RouterModesTab }
