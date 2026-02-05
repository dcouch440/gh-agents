import {useReducer} from 'react'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Box,
  Alert,
  Typography,
} from '@mui/material'
import {useCreateOutputSchema} from '@/hooks/useOutputSchemaMutations'
import {JsonEditor} from '@/components/primitives'
import {validateJsonObject} from '@/utils/json'
import {ApiError} from '@/api'

type OutputSchemaFormDialogProps = {
  open: boolean
  onClose: () => void
  onSave: (schemaId: string) => void
}

type FormState = {
  name: string
  description: string
  jsonSchemaText: string
  saving: boolean
  error: string | null
}

type FormAction =
  | {type: 'SET_NAME'; value: string}
  | {type: 'SET_DESCRIPTION'; value: string}
  | {type: 'SET_JSON_SCHEMA'; value: string}
  | {type: 'SET_SAVING'; value: boolean}
  | {type: 'SET_ERROR'; value: string | null}
  | {type: 'RESET'}

const initialFormState: FormState = {
  name: '',
  description: '',
  jsonSchemaText: '{\n  "type": "object",\n  "properties": {}\n}',
  saving: false,
  error: null,
}

const formReducer = (state: FormState, action: FormAction): FormState => {
  switch (action.type) {
    case 'SET_NAME':
      return {...state, name: action.value}
    case 'SET_DESCRIPTION':
      return {...state, description: action.value}
    case 'SET_JSON_SCHEMA':
      return {...state, jsonSchemaText: action.value}
    case 'SET_SAVING':
      return {...state, saving: action.value}
    case 'SET_ERROR':
      return {...state, error: action.value}
    case 'RESET':
      return initialFormState
    default:
      return state
  }
}

const validate = (state: FormState): string | null => {
  if (!state.name.trim()) return 'Name is required'
  if (state.name.length > 200) return 'Name must be 200 characters or less'

  const jsonResult = validateJsonObject(state.jsonSchemaText)
  if (!jsonResult.valid) return `Invalid JSON: ${jsonResult.error}`

  return null
}

function OutputSchemaFormDialog({open, onClose, onSave}: OutputSchemaFormDialogProps) {
  const [state, dispatch] = useReducer(formReducer, initialFormState)
  const {mutate} = useCreateOutputSchema()

  const handleSubmit = async () => {
    const error = validate(state)
    if (error) {
      dispatch({type: 'SET_ERROR', value: error})
      return
    }

    dispatch({type: 'SET_SAVING', value: true})
    dispatch({type: 'SET_ERROR', value: null})

    try {
      const parsed = JSON.parse(state.jsonSchemaText) as Record<string, unknown>
      const schema = await mutate({
        name: state.name.trim(),
        description: state.description.trim() || undefined,
        json_schema: parsed,
      })

      onSave(schema.id)
      onClose()
      dispatch({type: 'RESET'})
    } catch (err) {
      let errorMessage = 'Failed to create output schema'

      if (err instanceof ApiError && err.type === 'http_error') {
        if (err.status === 409) {
          errorMessage = 'A schema with this name already exists'
        } else {
          errorMessage = err.message
        }
      } else if (err instanceof Error) {
        errorMessage = err.message
      }

      dispatch({type: 'SET_ERROR', value: errorMessage})
    } finally {
      dispatch({type: 'SET_SAVING', value: false})
    }
  }

  const handleClose = () => {
    if (!state.saving) {
      onClose()
      dispatch({type: 'RESET'})
    }
  }

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="md" fullWidth>
      <DialogTitle>Create Output Schema</DialogTitle>
      <DialogContent dividers>
        {state.error && (
          <Alert severity="error" sx={{mb: 2}}>
            {state.error}
          </Alert>
        )}
        <Box sx={{display: 'flex', flexDirection: 'column', gap: 2}}>
          <TextField
            label="Name"
            value={state.name}
            onChange={(e) => dispatch({type: 'SET_NAME', value: e.target.value})}
            disabled={state.saving}
            required
            fullWidth
            inputProps={{maxLength: 200}}
            helperText={`${state.name.length}/200 characters`}
          />
          <TextField
            label="Description"
            value={state.description}
            onChange={(e) => dispatch({type: 'SET_DESCRIPTION', value: e.target.value})}
            disabled={state.saving}
            multiline
            rows={3}
            fullWidth
          />
          <Box>
            <Typography variant="body2" sx={{mb: 1, fontWeight: 500}}>
              JSON Schema
            </Typography>
            <JsonEditor
              value={state.jsonSchemaText}
              onChange={(value) => dispatch({type: 'SET_JSON_SCHEMA', value})}
              readOnly={state.saving}
              height="300px"
            />
            <Typography variant="caption" color="text.secondary" sx={{mt: 1, display: 'block'}}>
              Define a JSON Schema to structure the agent&apos;s output
            </Typography>
          </Box>
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={handleClose} disabled={state.saving}>
          Cancel
        </Button>
        <Button onClick={() => void handleSubmit()} variant="contained" disabled={state.saving}>
          Create
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export {OutputSchemaFormDialog}
export type {OutputSchemaFormDialogProps}
