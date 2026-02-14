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

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        const trimmed = value.trim()
        if (trimmed) {
          onSend(trimmed)
          setValue('')
        }
      }
    },
    [value, onSend],
  )

  return (
    <Box
      sx={{
        position: 'relative',
        px: 1.5,
        py: 1,
        '&::before': {
          content: '""',
          position: 'absolute',
          top: 0,
          left: 12,
          right: 12,
          height: '1px',
          bgcolor: 'divider',
          opacity: 0.5,
        },
      }}
    >
      <TextField
        fullWidth
        multiline
        maxRows={4}
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder={placeholder}
        variant="standard"
        size="small"
        sx={{
          '& .MuiInput-root': {
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
            '&::before': { borderBottom: 'none' },
            '&::after': { borderBottom: 'none' },
            '&:hover:not(.Mui-disabled)::before': { borderBottom: 'none' },
          },
          '& .MuiInput-input': {
            py: 0.75,
            '&::placeholder': { opacity: 0.4 },
          },
        }}
      />
    </Box>
  )
}

export { ChatInput }
export type { ChatInputProps }
