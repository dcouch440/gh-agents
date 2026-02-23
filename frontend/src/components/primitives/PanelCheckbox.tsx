import { Box, Typography } from '@mui/material'

type PanelCheckboxProps = {
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
  disabled?: boolean
}

function PanelCheckbox({ label, checked, onChange, disabled }: PanelCheckboxProps) {
  return (
    <Box
      onClick={disabled ? undefined : () => onChange(!checked)}
      sx={{
        display: 'flex',
        alignItems: 'baseline',
        gap: 0.75,
        py: 0.25,
        cursor: disabled ? 'default' : 'pointer',
        opacity: disabled ? 0.6 : 1,
        '&:hover': disabled ? {} : { bgcolor: 'action.hover' },
        borderRadius: 0.5,
        px: 0.5,
        mx: -0.5,
      }}
    >
      <Typography
        component="span"
        sx={{
          fontFamily: 'monospace',
          fontSize: '0.8125rem',
          lineHeight: 1.6,
          flexShrink: 0,
          userSelect: 'none',
        }}
      >
        [{checked ? 'X' : '\u00A0'}]
      </Typography>
      <Typography
        component="span"
        sx={{
          fontFamily: 'monospace',
          fontSize: '0.8125rem',
          lineHeight: 1.6,
        }}
      >
        {label}
      </Typography>
    </Box>
  )
}

export { PanelCheckbox }
export type { PanelCheckboxProps }
