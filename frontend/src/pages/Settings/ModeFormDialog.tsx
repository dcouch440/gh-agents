import { useReducer, useEffect, useMemo } from 'react'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Box,
  Alert,
  Checkbox,
  FormControlLabel,
  Divider,
  Typography,
} from '@mui/material'
import { useStore, toolStore, toolRouterStore } from '@/stores'
import { ToolCheckList } from '@/components/routers/ToolCheckList'
import { ApiError } from '@/api'
import { Collections } from '@/utils/collections'
import type { RouterMode } from '@/types'

type ModeFormDialogProps = {
  open: boolean
  onClose: () => void
  onSave: (mode: RouterMode) => void
  mode: RouterMode | null
  routerId: string
}

type FormState = {
  mode_key: string
  display_name: string
  description: string
  system_prompt: string
  temperature: number
  max_tokens: number
  append_to_agent_system_prompt: boolean
  append_to_agent_tools: boolean
  display_order: number
  selectedToolIds: string[]
  saving: boolean
  error: string | null
}

type FormAction =
  | { type: 'SET_MODE_KEY'; value: string }
  | { type: 'SET_DISPLAY_NAME'; value: string }
  | { type: 'SET_DESCRIPTION'; value: string }
  | { type: 'SET_SYSTEM_PROMPT'; value: string }
  | { type: 'SET_TEMPERATURE'; value: number }
  | { type: 'SET_MAX_TOKENS'; value: number }
  | { type: 'SET_APPEND_SYSTEM_PROMPT'; value: boolean }
  | { type: 'SET_APPEND_TOOLS'; value: boolean }
  | { type: 'SET_DISPLAY_ORDER'; value: number }
  | { type: 'SET_SELECTED_TOOL_IDS'; value: string[] }
  | { type: 'TOGGLE_TOOL'; toolId: string }
  | { type: 'SET_SAVING'; value: boolean }
  | { type: 'SET_ERROR'; value: string | null }
  | { type: 'HYDRATE'; payload: Omit<FormState, 'saving' | 'error' | 'selectedToolIds'> }
  | { type: 'RESET' }

const initialFormState: FormState = {
  mode_key: '',
  display_name: '',
  description: '',
  system_prompt: '',
  temperature: 0.7,
  max_tokens: 8192,
  append_to_agent_system_prompt: false,
  append_to_agent_tools: true,
  display_order: 0,
  selectedToolIds: [],
  saving: false,
  error: null,
}

const formReducer = (state: FormState, action: FormAction): FormState => {
  switch (action.type) {
    case 'SET_MODE_KEY':
      return { ...state, mode_key: action.value }
    case 'SET_DISPLAY_NAME':
      return { ...state, display_name: action.value }
    case 'SET_DESCRIPTION':
      return { ...state, description: action.value }
    case 'SET_SYSTEM_PROMPT':
      return { ...state, system_prompt: action.value }
    case 'SET_TEMPERATURE':
      return { ...state, temperature: action.value }
    case 'SET_MAX_TOKENS':
      return { ...state, max_tokens: action.value }
    case 'SET_APPEND_SYSTEM_PROMPT':
      return { ...state, append_to_agent_system_prompt: action.value }
    case 'SET_APPEND_TOOLS':
      return { ...state, append_to_agent_tools: action.value }
    case 'SET_DISPLAY_ORDER':
      return { ...state, display_order: action.value }
    case 'SET_SELECTED_TOOL_IDS':
      return { ...state, selectedToolIds: action.value }
    case 'TOGGLE_TOOL': {
      const idSet = Collections.toSet(state.selectedToolIds)
      return {
        ...state,
        selectedToolIds: idSet.has(action.toolId)
          ? Collections.filterMap(state.selectedToolIds, (id) => (id !== action.toolId ? id : null))
          : [...state.selectedToolIds, action.toolId],
      }
    }
    case 'SET_SAVING':
      return { ...state, saving: action.value }
    case 'SET_ERROR':
      return { ...state, error: action.value }
    case 'HYDRATE':
      return { ...action.payload, selectedToolIds: [], saving: false, error: null }
    case 'RESET':
      return initialFormState
  }
}

type ValidationError = { field: string; message: string }

const validate = (state: FormState, isEdit: boolean): ValidationError | null => {
  if (!isEdit) {
    if (!state.mode_key.trim()) {
      return { field: 'mode_key', message: 'Mode key is required' }
    }
    if (state.mode_key.length > 50) {
      return { field: 'mode_key', message: 'Mode key must be 50 characters or less' }
    }
    if (!/^[a-z][a-z0-9_]*$/.test(state.mode_key)) {
      return {
        field: 'mode_key',
        message: 'Mode key must start with a lowercase letter and contain only lowercase letters, numbers, and underscores',
      }
    }
  }

  if (!state.display_name.trim()) {
    return { field: 'display_name', message: 'Display name is required' }
  }
  if (state.display_name.length > 200) {
    return {
      field: 'display_name',
      message: 'Display name must be 200 characters or less',
    }
  }
  if (state.description.length > 10000) {
    return {
      field: 'description',
      message: 'Description must be 10,000 characters or less',
    }
  }
  if (state.temperature < 0 || state.temperature > 2) {
    return {
      field: 'temperature',
      message: 'Temperature must be between 0.0 and 2.0',
    }
  }
  if (state.max_tokens < 1 || state.max_tokens > 200000) {
    return {
      field: 'max_tokens',
      message: 'Max tokens must be between 1 and 200,000',
    }
  }

  return null
}

function ModeFormDialog({ open, onClose, onSave, mode, routerId }: ModeFormDialogProps) {
  const [state, dispatch] = useReducer(formReducer, initialFormState)
  const allTools = useStore(toolStore.store, toolStore.selectAll)

  useEffect(() => {
    void toolStore.fetchAll()
  }, [])
  const isEdit = mode !== null

  useEffect(() => {
    if (mode) {
      dispatch({
        type: 'HYDRATE',
        payload: {
          mode_key: mode.mode_key,
          display_name: mode.display_name,
          description: mode.description,
          system_prompt: mode.system_prompt,
          temperature: mode.temperature,
          max_tokens: mode.max_tokens,
          append_to_agent_system_prompt: mode.append_to_agent_system_prompt,
          append_to_agent_tools: mode.append_to_agent_tools,
          display_order: mode.display_order,
        },
      })
      let cancelled = false
      const load = async () => {
        try {
          const modeTools = await toolRouterStore.fetchModeTools(mode.id)
          if (!cancelled) {
            dispatch({ type: 'SET_SELECTED_TOOL_IDS', value: modeTools.map((t) => t.id) })
          }
        } catch {
          // Tools default to empty on error
        }
      }
      void load()
      return () => {
        cancelled = true
      }
    } else {
      dispatch({ type: 'RESET' })
    }
  }, [mode, open])

  const handleSubmit = async () => {
    const validationError = validate(state, isEdit)
    if (validationError) {
      dispatch({ type: 'SET_ERROR', value: validationError.message })
      return
    }

    dispatch({ type: 'SET_SAVING', value: true })
    dispatch({ type: 'SET_ERROR', value: null })

    try {
      let savedMode: RouterMode
      if (isEdit) {
        savedMode = await toolRouterStore.updateMode(mode.id, {
          display_name: state.display_name,
          description: state.description,
          system_prompt: state.system_prompt,
          temperature: state.temperature,
          max_tokens: state.max_tokens,
          append_to_agent_system_prompt: state.append_to_agent_system_prompt,
          append_to_agent_tools: state.append_to_agent_tools,
          display_order: state.display_order,
        })
      } else {
        savedMode = await toolRouterStore.createMode(routerId, {
          mode_key: state.mode_key,
          display_name: state.display_name,
          description: state.description,
          system_prompt: state.system_prompt,
          temperature: state.temperature,
          max_tokens: state.max_tokens,
          append_to_agent_system_prompt: state.append_to_agent_system_prompt,
          append_to_agent_tools: state.append_to_agent_tools,
          display_order: state.display_order,
        })
      }

      await toolRouterStore.setModeTools(savedMode.id, { tool_ids: state.selectedToolIds })

      onSave(savedMode)
      onClose()
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        dispatch({
          type: 'SET_ERROR',
          value: 'Mode key already exists. Please choose a different key.',
        })
      } else {
        dispatch({
          type: 'SET_ERROR',
          value: err instanceof Error ? err.message : 'Failed to save mode',
        })
      }
    } finally {
      dispatch({ type: 'SET_SAVING', value: false })
    }
  }

  const handleCancel = () => {
    dispatch({ type: 'RESET' })
    onClose()
  }

  const selectedIdSet = useMemo(() => Collections.toSet(state.selectedToolIds), [state.selectedToolIds])

  return (
    <Dialog open={open} onClose={handleCancel} maxWidth="md" fullWidth>
      <DialogTitle>{isEdit ? 'Edit Router Mode' : 'Create Router Mode'}</DialogTitle>

      <DialogContent dividers>
        {state.error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {state.error}
          </Alert>
        )}

        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          <TextField
            label="Mode Key"
            value={state.mode_key}
            onChange={(e) => dispatch({ type: 'SET_MODE_KEY', value: e.target.value })}
            disabled={isEdit || state.saving}
            required
            fullWidth
            helperText={isEdit ? 'Mode key cannot be changed' : 'Lowercase letters, numbers, and underscores only'}
          />

          <TextField
            label="Display Name"
            value={state.display_name}
            onChange={(e) => dispatch({ type: 'SET_DISPLAY_NAME', value: e.target.value })}
            disabled={state.saving}
            required
            fullWidth
          />

          <TextField
            label="Description"
            value={state.description}
            onChange={(e) => dispatch({ type: 'SET_DESCRIPTION', value: e.target.value })}
            disabled={state.saving}
            multiline
            rows={3}
            fullWidth
          />

          <TextField
            label="System Prompt"
            value={state.system_prompt}
            onChange={(e) => dispatch({ type: 'SET_SYSTEM_PROMPT', value: e.target.value })}
            disabled={state.saving}
            multiline
            rows={5}
            fullWidth
          />

          <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 2 }}>
            <TextField
              label="Temperature"
              type="number"
              value={state.temperature}
              onChange={(e) =>
                dispatch({
                  type: 'SET_TEMPERATURE',
                  value: Number(e.target.value),
                })
              }
              disabled={state.saving}
              inputProps={{ min: 0, max: 2, step: 0.1 }}
              fullWidth
            />

            <TextField
              label="Max Tokens"
              type="number"
              value={state.max_tokens}
              onChange={(e) =>
                dispatch({
                  type: 'SET_MAX_TOKENS',
                  value: Number(e.target.value),
                })
              }
              disabled={state.saving}
              inputProps={{ min: 1, max: 200000, step: 1 }}
              fullWidth
            />
          </Box>

          <FormControlLabel
            control={
              <Checkbox
                checked={state.append_to_agent_system_prompt}
                onChange={(e) =>
                  dispatch({
                    type: 'SET_APPEND_SYSTEM_PROMPT',
                    value: e.target.checked,
                  })
                }
                disabled={state.saving}
              />
            }
            label="Append to agent system prompt"
          />

          <FormControlLabel
            control={
              <Checkbox
                checked={state.append_to_agent_tools}
                onChange={(e) =>
                  dispatch({
                    type: 'SET_APPEND_TOOLS',
                    value: e.target.checked,
                  })
                }
                disabled={state.saving}
              />
            }
            label="Append to agent tools"
          />

          <TextField
            label="Display Order"
            type="number"
            value={state.display_order}
            onChange={(e) =>
              dispatch({
                type: 'SET_DISPLAY_ORDER',
                value: Number(e.target.value),
              })
            }
            disabled={state.saving}
            inputProps={{ min: 0, step: 1 }}
            fullWidth
            helperText="Lower numbers appear first"
          />

          <Divider />

          <Typography variant="subtitle2">Tools ({state.selectedToolIds.length} selected)</Typography>
          {allTools.length === 0 ? (
            <Typography variant="body2" color="text.secondary">
              No tools available
            </Typography>
          ) : (
            <ToolCheckList
              tools={allTools}
              selectedIds={selectedIdSet}
              onToggle={(toolId) => dispatch({ type: 'TOGGLE_TOOL', toolId })}
              disabled={state.saving}
            />
          )}
        </Box>
      </DialogContent>

      <DialogActions>
        <Button onClick={handleCancel} disabled={state.saving}>
          Cancel
        </Button>
        <Button
          onClick={() => {
            void handleSubmit()
          }}
          variant="contained"
          disabled={state.saving}
        >
          {state.saving ? 'Saving...' : isEdit ? 'Update' : 'Create'}
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export { ModeFormDialog }
export type { ModeFormDialogProps }
