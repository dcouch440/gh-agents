import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import InputBase from '@mui/material/InputBase'

const inputSx = {
  fontSize: 13,
  px: 1,
  py: 0.5,
  borderRadius: '6px',
  border: 1,
  borderColor: 'divider',
  '&:hover': { borderColor: 'text.disabled' },
  '&.Mui-focused': { borderColor: 'primary.main' },
  transition: 'border-color 150ms ease',
} as const

type SettingsTabProps = {
  name: string
  onNameChange: (value: string) => void
  description: string
  onDescriptionChange: (value: string) => void
}

function SettingsTab({ name, onNameChange, description, onDescriptionChange }: SettingsTabProps) {
  return (
    <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1.5 }}>
      <Box>
        <Typography sx={{ fontSize: 11, fontWeight: 600, color: 'text.secondary', mb: 0.5 }}>Name</Typography>
        <InputBase
          value={name}
          onChange={(e) => {
            onNameChange(e.target.value)
          }}
          placeholder="Documenter"
          fullWidth
          sx={inputSx}
        />
      </Box>
      <Box>
        <Typography sx={{ fontSize: 11, fontWeight: 600, color: 'text.secondary', mb: 0.5 }}>Description</Typography>
        <InputBase
          value={description}
          onChange={(e) => {
            onDescriptionChange(e.target.value)
          }}
          placeholder="Describe what this step does..."
          fullWidth
          multiline
          minRows={2}
          sx={inputSx}
        />
      </Box>
    </Box>
  )
}

export { SettingsTab }
export type { SettingsTabProps }
