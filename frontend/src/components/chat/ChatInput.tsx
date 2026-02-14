import { useId, useState, useCallback } from 'react'
import { Box, TextField } from '@mui/material'

type ChatInputProps = {
  onSend: (message: string) => void
  disabled?: boolean
  placeholder?: string
  inputRef?: React.RefObject<HTMLTextAreaElement | null>
}

function ChatInput({ onSend, disabled, placeholder = 'Type a message...', inputRef }: ChatInputProps) {
  const inputId = useId()
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
      component="label"
      htmlFor={inputId}
      sx={{
        display: 'block',
        position: 'relative',
        px: 1.5,
        py: 1,
        cursor: 'text',
      }}
    >
      <TextField
        fullWidth
        multiline
        maxRows={4}
        id={inputId}
        inputRef={inputRef}
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
