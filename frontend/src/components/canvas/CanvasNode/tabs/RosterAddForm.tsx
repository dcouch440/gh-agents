import { useState } from 'react'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import InputBase from '@mui/material/InputBase'
import { useTheme } from '@mui/material/styles'

type RosterAddFormProps = {
  onSubmit: (name: string, roleDescription: string) => void
  onCancel: () => void
}

function RosterAddForm({ onSubmit, onCancel }: RosterAddFormProps) {
  const theme = useTheme()
  const [name, setName] = useState('')
  const [roleDescription, setRoleDescription] = useState('')

  const isValid = name.trim().length > 0

  const handleSubmit = () => {
    if (!isValid) return
    onSubmit(name, roleDescription)
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
        onChange={(e) => { setName(e.target.value) }}
        onKeyDown={handleKeyDown}
        placeholder="Agent name"
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
        value={roleDescription}
        onChange={(e) => { setRoleDescription(e.target.value) }}
        onKeyDown={handleKeyDown}
        placeholder="Role description (optional)"
        multiline
        rows={2}
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
      <Box sx={{ display: 'flex', gap: 0.5, justifyContent: 'flex-end' }}>
        <Button size="small" onClick={onCancel} sx={{ fontSize: 11, textTransform: 'none', minWidth: 0, px: 1 }}>
          Cancel
        </Button>
        <Button
          size="small"
          variant="contained"
          onClick={handleSubmit}
          disabled={!isValid}
          sx={{ fontSize: 11, textTransform: 'none', minWidth: 0, px: 1 }}
        >
          Add
        </Button>
      </Box>
    </Box>
  )
}

export { RosterAddForm }
export type { RosterAddFormProps }
