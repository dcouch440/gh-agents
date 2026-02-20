import { useState } from 'react'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import InputBase from '@mui/material/InputBase'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import type { CreateDocumentDefRequest } from '@/types/workflow'

type InlineAddFormProps = {
  onSubmit: (body: CreateDocumentDefRequest) => void
  onCancel: () => void
}

function InlineAddForm({ onSubmit, onCancel }: InlineAddFormProps) {
  const theme = useTheme()
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [targetLength, setTargetLength] = useState(1000)

  const isValid = name.trim().length > 0

  const handleSubmit = () => {
    if (!isValid) return
    onSubmit({
      name: name.trim(),
      description: description.trim() || undefined,
      target_length: targetLength,
    })
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && isValid) {
      e.preventDefault()
      handleSubmit()
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      onCancel()
    }
  }

  return (
    <Box
      sx={{
        p: 1,
        borderRadius: '8px',
        border: 1,
        borderColor: 'primary.main',
        backgroundColor: theme.palette.custom.hoverOverlay,
        display: 'flex',
        flexDirection: 'column',
        gap: 0.75,
      }}
    >
      <InputBase
        autoFocus
        value={name}
        onChange={(e) => {
          setName(e.target.value)
        }}
        onKeyDown={handleKeyDown}
        placeholder="Document name"
        inputProps={{ 'data-testid': 'inline-add-name' }}
        sx={{
          fontSize: 12,
          fontWeight: 600,
          px: 0.75,
          py: 0.25,
          borderRadius: '4px',
          border: 1,
          borderColor: 'divider',
          backgroundColor: 'background.paper',
        }}
      />
      <InputBase
        value={description}
        onChange={(e) => {
          setDescription(e.target.value)
        }}
        onKeyDown={handleKeyDown}
        placeholder="Description (optional)"
        multiline
        rows={2}
        inputProps={{ 'data-testid': 'inline-add-description' }}
        sx={{
          fontSize: 11,
          px: 0.75,
          py: 0.25,
          borderRadius: '4px',
          border: 1,
          borderColor: 'divider',
          backgroundColor: 'background.paper',
        }}
      />
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
        <Typography sx={{ fontSize: 10, color: 'text.secondary', whiteSpace: 'nowrap' }}>Target length:</Typography>
        <InputBase
          value={targetLength}
          onChange={(e) => {
            const n = parseInt(e.target.value, 10)
            if (!isNaN(n) && n >= 0) setTargetLength(n)
          }}
          onKeyDown={handleKeyDown}
          type="number"
          inputProps={{ 'data-testid': 'inline-add-target-length' }}
          sx={{
            fontSize: 11,
            px: 0.5,
            py: 0.25,
            borderRadius: '4px',
            border: 1,
            borderColor: 'divider',
            backgroundColor: 'background.paper',
            width: 70,
            '& input': { textAlign: 'right' },
          }}
        />
      </Box>
      <Box sx={{ display: 'flex', gap: 0.5, justifyContent: 'flex-end' }}>
        <Button size="small" onClick={onCancel} sx={{ fontSize: 11, textTransform: 'none', minWidth: 0, px: 1 }}>
          Cancel
        </Button>
        <Button
          size="small"
          variant="contained"
          onClick={handleSubmit}
          disabled={!isValid}
          data-testid="inline-add-submit"
          sx={{ fontSize: 11, textTransform: 'none', minWidth: 0, px: 1 }}
        >
          Add
        </Button>
      </Box>
    </Box>
  )
}

export { InlineAddForm }
export type { InlineAddFormProps }
