import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import InputBase from '@mui/material/InputBase'

type SettingsTabProps = {
  name: string
  onNameChange: (value: string) => void
}

function SettingsTab({ name, onNameChange }: SettingsTabProps) {
  return (
    <Box sx={{ p: 1.5 }}>
      <Typography sx={{ fontSize: 11, fontWeight: 600, color: 'text.secondary', mb: 0.5 }}>Name</Typography>
      <InputBase
        value={name}
        onChange={(e) => {
          onNameChange(e.target.value)
        }}
        placeholder="Documenter"
        fullWidth
        sx={{
          fontSize: 13,
          px: 1,
          py: 0.5,
          borderRadius: '6px',
          border: 1,
          borderColor: 'divider',
          '&:hover': { borderColor: 'text.disabled' },
          '&.Mui-focused': { borderColor: 'primary.main' },
          transition: 'border-color 150ms ease',
        }}
      />
    </Box>
  )
}

export { SettingsTab }
export type { SettingsTabProps }
