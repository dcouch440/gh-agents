import { useState } from 'react'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Checkbox,
  FormControlLabel,
  Box,
  Typography,
} from '@mui/material'
import { Button, LoadingSpinner } from '@/components/primitives'
import type { Tool } from '@/types'

type ToolAssignmentDialogProps = {
  open: boolean
  onClose: () => void
  onSave: (toolIds: string[]) => void
  allTools: Tool[]
  assignedToolIds: string[]
  saving: boolean
  loading: boolean
  title: string
}

function ToolAssignmentContent({
  onClose,
  onSave,
  allTools,
  assignedToolIds,
  saving,
  loading,
}: {
  onClose: () => void
  onSave: (toolIds: string[]) => void
  allTools: Tool[]
  assignedToolIds: string[]
  saving: boolean
  loading: boolean
}) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set(assignedToolIds))

  const handleToggle = (toolId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(toolId)) next.delete(toolId)
      else next.add(toolId)
      return next
    })
  }

  return (
    <>
      <DialogContent dividers>
        {loading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
            <LoadingSpinner size="md" />
          </Box>
        ) : allTools.length === 0 ? (
          <Typography variant="body2" color="text.secondary" sx={{ py: 2 }}>
            No tools available
          </Typography>
        ) : (
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
            {allTools.map((tool) => (
              <FormControlLabel
                key={tool.id}
                control={
                  <Checkbox
                    checked={selectedIds.has(tool.id)}
                    onChange={() => handleToggle(tool.id)}
                    size="small"
                  />
                }
                label={
                  <Box>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      {tool.name}
                    </Typography>
                    {tool.description ? (
                      <Typography variant="caption" color="text.secondary">
                        {tool.description}
                      </Typography>
                    ) : null}
                  </Box>
                }
              />
            ))}
          </Box>
        )}
      </DialogContent>
      <DialogActions sx={{ px: 3, py: 2 }}>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={() => onSave([...selectedIds])} disabled={saving || loading}>
          {saving ? 'Saving...' : 'Save'}
        </Button>
      </DialogActions>
    </>
  )
}

function ToolAssignmentDialog({
  open,
  onClose,
  onSave,
  allTools,
  assignedToolIds,
  saving,
  loading,
  title,
}: ToolAssignmentDialogProps) {
  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{title}</DialogTitle>
      {open ? (
        <ToolAssignmentContent
          onClose={onClose}
          onSave={onSave}
          allTools={allTools}
          assignedToolIds={assignedToolIds}
          saving={saving}
          loading={loading}
        />
      ) : null}
    </Dialog>
  )
}

export { ToolAssignmentDialog }
export type { ToolAssignmentDialogProps }
