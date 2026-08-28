import { useId, useState, useCallback, useEffect } from 'react'
import { Box, IconButton, TextField } from '@mui/material'
import StopCircleIcon from '@mui/icons-material/StopCircle'

type ChatInputProps = {
  onSend: (message: string) => void
  /** When set together with `disabled`, shows a stop button and binds Escape. */
  onCancel?: () => void
  disabled?: boolean
  placeholder?: string
  inputRef?: React.RefObject<HTMLTextAreaElement | null>
}

function ChatInput({ onSend, onCancel, disabled, placeholder = 'Type a message...', inputRef }: ChatInputProps) {
  const inputId = useId()
  const [value, setValue] = useState('')

  const canCancel = Boolean(disabled && onCancel)

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

  // A disabled textarea never sees key events, so Escape is bound on the
  // document while a turn is in flight — that is the only time it is live.
  useEffect(() => {
    if (!canCancel || !onCancel) return
    const handler = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      e.preventDefault()
      onCancel()
    }
    document.addEventListener('keydown', handler)
    return () => { document.removeEventListener('keydown', handler) }
  }, [canCancel, onCancel])

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
        placeholder={canCancel ? 'Working — press Esc to stop' : placeholder}
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
            pr: canCancel ? 4 : 0,
            '&::placeholder': { opacity: 0.4 },
          },
        }}
      />

      {canCancel && onCancel ? (
        <IconButton
          onClick={onCancel}
          size="small"
          aria-label="Stop generation (Esc)"
          sx={{
            position: 'absolute',
            right: 4,
            top: '50%',
            transform: 'translateY(-50%)',
            color: 'text.secondary',
            '&:hover': { color: 'error.main' },
          }}
        >
          <StopCircleIcon fontSize="small" />
        </IconButton>
      ) : null}
    </Box>
  )
}

export { ChatInput }
export type { ChatInputProps }
