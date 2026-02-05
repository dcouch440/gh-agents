import { useState } from 'react'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Box,
  Slider,
  Typography,
  Switch,
  FormControlLabel,
  Checkbox,
  Divider,
} from '@mui/material'
import { Button } from '@/components/primitives'
import type { CreateRouterModeRequest, Tool } from '@/types'

type ModeFormDialogProps = {
  open: boolean
  onClose: () => void
  onSubmit: (data: CreateRouterModeRequest, toolIds: string[]) => void
  initialValues: Partial<CreateRouterModeRequest> | null
  allTools: Tool[]
  initialToolIds: string[]
  saving: boolean
  title: string
}

function ModeFormContent({
  onClose,
  onSubmit,
  initialValues,
  allTools,
  initialToolIds,
  saving,
}: {
  onClose: () => void
  onSubmit: (data: CreateRouterModeRequest, toolIds: string[]) => void
  initialValues: Partial<CreateRouterModeRequest> | null
  allTools: Tool[]
  initialToolIds: string[]
  saving: boolean
}) {
  const [modeKey, setModeKey] = useState(initialValues?.mode_key ?? '')
  const [displayName, setDisplayName] = useState(initialValues?.display_name ?? '')
  const [description, setDescription] = useState(initialValues?.description ?? '')
  const [systemPrompt, setSystemPrompt] = useState(initialValues?.system_prompt ?? '')
  const [temperature, setTemperature] = useState(initialValues?.temperature ?? 0.7)
  const [maxTokens, setMaxTokens] = useState(initialValues?.max_tokens ?? 4096)
  const [appendPrompt, setAppendPrompt] = useState(initialValues?.append_to_agent_system_prompt ?? false)
  const [appendTools, setAppendTools] = useState(initialValues?.append_to_agent_tools ?? true)
  const [displayOrder, setDisplayOrder] = useState(initialValues?.display_order ?? 0)
  const [selectedToolIds, setSelectedToolIds] = useState<string[]>([...initialToolIds])

  const handleToggleTool = (toolId: string) => {
    setSelectedToolIds((prev) =>
      prev.includes(toolId) ? prev.filter((id) => id !== toolId) : [...prev, toolId],
    )
  }

  const handleSubmit = () => {
    onSubmit(
      {
        mode_key: modeKey.trim(),
        display_name: displayName.trim(),
        description: description.trim(),
        system_prompt: systemPrompt,
        temperature,
        max_tokens: maxTokens,
        append_to_agent_system_prompt: appendPrompt,
        append_to_agent_tools: appendTools,
        display_order: displayOrder,
      },
      selectedToolIds,
    )
  }

  const isValid =
    modeKey.trim().length > 0 &&
    displayName.trim().length > 0 &&
    description.trim().length > 0

  return (
    <>
      <DialogContent dividers>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, pt: 1 }}>
          <TextField
            label="Mode Key"
            value={modeKey}
            onChange={(e) => setModeKey(e.target.value)}
            fullWidth
            required
            size="small"
            helperText="Snake case (e.g. planning_mode)"
          />
          <TextField
            label="Display Name"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            fullWidth
            required
            size="small"
          />
          <TextField
            label="Description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            fullWidth
            required
            multiline
            rows={2}
            size="small"
            helperText="Used by the LLM to classify which mode to select"
          />
          <TextField
            label="System Prompt"
            value={systemPrompt}
            onChange={(e) => setSystemPrompt(e.target.value)}
            fullWidth
            multiline
            rows={4}
            size="small"
          />

          <Box>
            <Typography variant="caption" color="text.secondary" gutterBottom>
              Temperature: {temperature.toFixed(1)}
            </Typography>
            <Slider
              value={temperature}
              onChange={(_, v) => { if (typeof v === 'number') setTemperature(v) }}
              min={0}
              max={2}
              step={0.1}
              size="small"
            />
          </Box>

          <TextField
            label="Max Tokens"
            type="number"
            value={maxTokens}
            onChange={(e) => setMaxTokens(Number(e.target.value))}
            fullWidth
            size="small"
          />

          <TextField
            label="Display Order"
            type="number"
            value={displayOrder}
            onChange={(e) => setDisplayOrder(Number(e.target.value))}
            fullWidth
            size="small"
          />

          <FormControlLabel
            control={
              <Switch checked={appendPrompt} onChange={(e) => setAppendPrompt(e.target.checked)} />
            }
            label="Append to agent system prompt"
          />

          <FormControlLabel
            control={
              <Switch checked={appendTools} onChange={(e) => setAppendTools(e.target.checked)} />
            }
            label="Append to agent tools"
          />

          <Divider />

          <Typography variant="subtitle2">Tools</Typography>
          {allTools.length === 0 ? (
            <Typography variant="body2" color="text.secondary">
              No tools available
            </Typography>
          ) : (
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
              {allTools.map((tool) => (
                <FormControlLabel
                  key={tool.id}
                  control={
                    <Checkbox
                      checked={selectedToolIds.includes(tool.id)}
                      onChange={() => handleToggleTool(tool.id)}
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

function ModeFormDialog({ open, onClose, onSubmit, initialValues, allTools, initialToolIds, saving, title }: ModeFormDialogProps) {
  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{title}</DialogTitle>
      {open ? (
        <ModeFormContent
          onClose={onClose}
          onSubmit={onSubmit}
          initialValues={initialValues}
          allTools={allTools}
          initialToolIds={initialToolIds}
          saving={saving}
        />
      ) : null}
    </Dialog>
  )
}

export { ModeFormDialog }
export type { ModeFormDialogProps }
