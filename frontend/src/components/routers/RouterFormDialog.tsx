import { useState } from 'react'
import { Dialog, DialogTitle, DialogContent, DialogActions, TextField, Box } from '@mui/material'
import { Button } from '@/components/primitives'
import type { CreateToolRouterRequest } from '@/types'

type RouterFormDialogProps = {
  open: boolean
  onClose: () => void
  onSubmit: (data: CreateToolRouterRequest) => void
  initialValues: CreateToolRouterRequest | null
  saving: boolean
  title: string
}

function RouterFormContent({
  onClose,
  onSubmit,
  initialValues,
  saving,
}: {
  onClose: () => void
  onSubmit: (data: CreateToolRouterRequest) => void
  initialValues: CreateToolRouterRequest | null
  saving: boolean
}) {
  const [name, setName] = useState(initialValues?.name ?? '')
  const [description, setDescription] = useState(initialValues?.description ?? '')
  const [systemPrompt, setSystemPrompt] = useState(initialValues?.system_prompt ?? '')
  const [modelId, setModelId] = useState(initialValues?.model_id ?? 'claude-sonnet-4-20250514')

  const handleSubmit = () => {
    const data: CreateToolRouterRequest = {
      name: name.trim(),
      system_prompt: systemPrompt,
      model_id: modelId.trim(),
    }
    if (description.trim()) {
      data.description = description.trim()
    }
    onSubmit(data)
  }

  const isValid = name.trim().length > 0 && modelId.trim().length > 0

  return (
    <>
      <DialogContent dividers>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, pt: 1 }}>
          <TextField label="Name" value={name} onChange={(e) => setName(e.target.value)} fullWidth required size="small" />
          <TextField label="Description" value={description} onChange={(e) => setDescription(e.target.value)} fullWidth size="small" />
          <TextField label="Model ID" value={modelId} onChange={(e) => setModelId(e.target.value)} fullWidth required size="small" />
          <TextField
            label="System Prompt"
            value={systemPrompt}
            onChange={(e) => setSystemPrompt(e.target.value)}
            fullWidth
            multiline
            rows={4}
            size="small"
          />
        </Box>
      </DialogContent>
      <DialogActions sx={{ px: 3, py: 2 }}>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleSubmit} disabled={saving || !isValid}>
          {saving ? 'Saving...' : 'Save'}
        </Button>
      </DialogActions>
    </>
  )
}

function RouterFormDialog({ open, onClose, onSubmit, initialValues, saving, title }: RouterFormDialogProps) {
  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{title}</DialogTitle>
      {open ? <RouterFormContent onClose={onClose} onSubmit={onSubmit} initialValues={initialValues} saving={saving} /> : null}
    </Dialog>
  )
}

export { RouterFormDialog }
export type { RouterFormDialogProps }
