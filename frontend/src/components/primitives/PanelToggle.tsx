import Box from '@mui/material/Box'
import Switch from '@mui/material/Switch'

type PanelToggleProps = {
  checked: boolean
  onChange: (checked: boolean) => void
}

function PanelToggle({ checked, onChange }: PanelToggleProps) {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center' }}>
      <Switch
        checked={checked}
        onChange={(_, value) => { onChange(value) }}
        size="small"
        sx={{
          width: 28,
          height: 16,
          padding: 0,
          '& .MuiSwitch-switchBase': {
            padding: '2px',
            '&.Mui-checked': {
              transform: 'translateX(12px)',
              '& + .MuiSwitch-track': {
                backgroundColor: 'primary.main',
                opacity: 1,
              },
            },
          },
          '& .MuiSwitch-thumb': {
            width: 12,
            height: 12,
            boxShadow: 'none',
          },
          '& .MuiSwitch-track': {
            borderRadius: 8,
            opacity: 0.3,
          },
        }}
      />
    </Box>
  )
}

export { PanelToggle }
export type { PanelToggleProps }
