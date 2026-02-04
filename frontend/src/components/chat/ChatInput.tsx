import { useState, useCallback } from 'react'
import { Box, TextField } from '@mui/material'

type ChatInputProps = {
  onSend: (message: string) => void
  disabled?: boolean
  placeholder?: string
}

function ChatInput({ onSend, disabled, placeholder = 'Type a message...' }: ChatInputProps) {
  const [value, setValue] = useState('')

  const handleChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setValue(e.target.value)
  }, [])

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      const trimmed = value.trim()
      if (trimmed) {
        onSend(trimmed)
        setValue('')
      }
    }
  }, [value, onSend])

  return (
    <Box sx={{ borderTop: 1, borderColor: 'divider', p: 1.5, bgcolor: 'background.paper' }}>
      <TextField
        fullWidth
        multiline
        maxRows={6}
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder={placeholder}
        variant="outlined"
        size="small"
        sx={{
          '& .MuiOutlinedInput-root': {
            fontFamily: 'monospace',
            fontSize: '0.875rem',
          },
        }}
      />
    </Box>
  )
}

export { ChatInput }
export type { ChatInputProps }
