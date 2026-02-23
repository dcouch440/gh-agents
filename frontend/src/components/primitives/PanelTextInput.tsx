import { Box, InputBase, Typography } from '@mui/material'

type PanelTextInputProps = {
  label: string
  value: string
  onChange: (value: string) => void
  disabled?: boolean
}

function PanelTextInput({ label, value, onChange, disabled }: PanelTextInputProps) {
  if (disabled && !value) return null

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'baseline',
        gap: 0.75,
        py: 0.25,
        px: 0.5,
        mx: -0.5,
        opacity: disabled ? 0.6 : 1,
      }}
    >
      {label ? (
        <Typography
          component="span"
          sx={{
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
            lineHeight: 1.6,
            flexShrink: 0,
            color: 'text.secondary',
            whiteSpace: 'nowrap',
          }}
        >
          {label}:
        </Typography>
      ) : null}
      {disabled ? (
        <Typography
          component="span"
          sx={{
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
            lineHeight: 1.6,
            whiteSpace: 'pre-wrap',
          }}
        >
          {value}
        </Typography>
      ) : (
        <InputBase
          fullWidth
          multiline
          maxRows={6}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="..."
          sx={{
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
            lineHeight: 1.6,
            p: 0,
            '& .MuiInputBase-input': {
              p: 0,
              '&::placeholder': { opacity: 0.35 },
            },
          }}
        />
      )}
    </Box>
  )
}

export { PanelTextInput }
export type { PanelTextInputProps }
