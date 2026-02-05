import { useState, useEffect } from 'react'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Checkbox,
  Typography,
  Box,
  CircularProgress,
  Alert,
} from '@mui/material'
import { EmptyState } from '@/components/primitives'
import type { RouterMode, Tool, SetModeToolsRequest } from '@/types'

type ModeToolSelectorProps = {
  open: boolean
  onClose: () => void
  mode: RouterMode
  allTools: Tool[]
  loadModeTools: (modeId: string) => Promise<Tool[]>
  saveModeTools: (modeId: string, body: SetModeToolsRequest) => Promise<void>
  loadingTools: boolean
  savingTools: boolean
  toolsError: string | null
}

function ModeToolSelector({
  open,
  onClose,
  mode,
  allTools,
  loadModeTools,
  saveModeTools,
  loadingTools,
  savingTools,
  toolsError,
}: ModeToolSelectorProps) {
  const [localSelectedIds, setLocalSelectedIds] = useState<string[]>([])
  const [originalIds, setOriginalIds] = useState<string[]>([])

  useEffect(() => {
    if (!open) return

    let cancelled = false
    const load = async () => {
      try {
        const modeTools = await loadModeTools(mode.id)
        if (cancelled) return
        const ids = modeTools.map((t) => t.id)
        setLocalSelectedIds(ids)
        setOriginalIds(ids)
      } catch {
        // Error handled by mutation hook (toolsError)
      }
    }
    void load()

    return () => {
      cancelled = true
    }
  }, [open, mode.id, loadModeTools])

  const handleToggle = (toolId: string) => {
    setLocalSelectedIds((prev) =>
      prev.includes(toolId)
        ? prev.filter((id) => id !== toolId)
        : [...prev, toolId]
    )
  }

  const handleSave = async () => {
    try {
      await saveModeTools(mode.id, { tool_ids: localSelectedIds })
      onClose()
    } catch {
      // Error handled by mutation hook (toolsError state)
    }
  }

  const handleCancel = () => {
    setLocalSelectedIds(originalIds)
    onClose()
  }

  if (!open) return null

  return (
    <Dialog open={open} onClose={handleCancel} maxWidth="md" fullWidth>
      <DialogTitle>
        <Typography variant="h6" component="div" sx={{ fontWeight: 600 }}>
          Select Tools for {mode.display_name}
        </Typography>
        <Typography variant="body2" color="text.secondary">
          {localSelectedIds.length} tool{localSelectedIds.length === 1 ? '' : 's'}{' '}
          selected
        </Typography>
      </DialogTitle>

      <DialogContent dividers sx={{ p: 0, bgcolor: 'background.default' }}>
        {toolsError && (
          <Alert severity="error" sx={{ m: 2 }}>
            {toolsError}
          </Alert>
        )}

        {loadingTools ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 7.5 }}>
            <CircularProgress />
          </Box>
        ) : allTools.length === 0 ? (
          <Box sx={{ p: 5 }}>
            <EmptyState message="No tools available" />
          </Box>
        ) : (
          <List disablePadding>
            {allTools.map((tool) => {
              const isSelected = localSelectedIds.includes(tool.id)
              return (
                <ListItem
                  key={tool.id}
                  disablePadding
                  sx={{ borderBottom: 1, borderColor: 'divider' }}
                >
                  <ListItemButton
                    onClick={() => handleToggle(tool.id)}
                    disabled={savingTools}
                  >
                    <ListItemIcon sx={{ minWidth: 36 }}>
                      <Checkbox
                        checked={isSelected}
                        edge="start"
                        tabIndex={-1}
                        disableRipple
                        disabled={savingTools}
                      />
                    </ListItemIcon>
                    <ListItemText
                      primary={
                        <Typography variant="body2" sx={{ fontWeight: 500 }}>
                          {tool.name}
                        </Typography>
                      }
                      secondary={
                        <Typography variant="caption" color="text.secondary">
                          {tool.description}
                        </Typography>
                      }
                    />
                  </ListItemButton>
                </ListItem>
              )
            })}
          </List>
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={handleCancel} disabled={savingTools}>
          Cancel
        </Button>
        <Button
          onClick={() => {
            void handleSave()
          }}
          variant="contained"
          disabled={savingTools || loadingTools}
        >
          {savingTools ? 'Saving...' : 'Save'}
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export { ModeToolSelector }
export type { ModeToolSelectorProps }
