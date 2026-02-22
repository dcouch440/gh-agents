import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'

function HorizontalRuleRenderer() {
  const theme = useTerminalTheme()

  return (
    <Box
      component="hr"
      sx={{
        border: 'none',
        borderTop: `1px solid ${theme.divider}`,
        my: '0.6em',
      }}
    />
  )
}

export { HorizontalRuleRenderer }
