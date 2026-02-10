import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

function ActivityTab() {
  return (
    <Box sx={{ p: 1.5, height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <Typography sx={{ fontSize: 12, color: 'text.disabled' }}>Coming soon</Typography>
    </Box>
  )
}

export { ActivityTab }
