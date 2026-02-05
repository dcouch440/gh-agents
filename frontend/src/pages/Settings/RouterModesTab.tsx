import { useState, useEffect, useCallback } from 'react'
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
import { useToolRouterMutations } from '@/hooks/useToolRouterMutations'
import { api } from '@/api'
import type { RouterMode, ToolRouter } from '@/types'

function RouterModesTab() {
  const [routers, setRouters] = useState<ToolRouter[]>([])
  const [routersLoading, setRoutersLoading] = useState(true)
  const [routersError, setRoutersError] = useState<string | null>(null)
  const routerMutations = useToolRouterMutations()

  const loadRouters = useCallback(async () => {
    setRoutersLoading(true)
    setRoutersError(null)
    try {
      const data = await api.toolRouters.list()
      setRouters(data)
    } catch (e) {
      setRoutersError(e instanceof Error ? e.message : 'Failed to load routers')
    } finally {
      setRoutersLoading(false)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      await loadRouters()
      if (cancelled) return
    }
    void run()
    return () => { cancelled = true }
  }, [loadRouters])

  const activeRouter = routers.length > 0 ? routers[0] : null
  const activeRouterId = activeRouter?.id ?? null

  const handleCreateRouter = async () => {
    const newRouter = await routerMutations.createRouter({
      name: 'Default Router',
      description: 'Auto-created default router for mode management',
      system_prompt: 'You are a routing assistant. Analyze the user message and select the most appropriate mode.',
      model_id: 'claude-sonnet-4-20250514',
    })
    setRouters([newRouter])
  }

  const { modes, loading, error, reload } = useRouterModes(activeRouterId)
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

  if (routersLoading) {
    return (
      <Box>
        <PageHeader title="Router Modes" />
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
          <CircularProgress />
        </Box>
      </Box>
    )
  }

  if (routersError) {
    return (
      <Box>
        <PageHeader title="Router Modes" />
        <Alert severity="error">{routersError}</Alert>
      </Box>
    )
  }

  if (!activeRouter) {
    return (
      <Box>
        <PageHeader title="Router Modes" />
        <Box sx={{ textAlign: 'center', py: 4 }}>
          <Typography variant="body1" color="text.secondary" sx={{ mb: 2 }}>
            No tool router exists yet. Create one to start configuring modes.
          </Typography>
          <Button
            variant="contained"
            onClick={() => { void handleCreateRouter() }}
            disabled={routerMutations.creating}
          >
            {routerMutations.creating ? 'Creating...' : 'Create Default Router'}
          </Button>
        </Box>
      </Box>
    )
  }

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
        routerId={activeRouter.id}
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
