import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type MetricChipProps = {
  label: string
  value: string
}

function MetricChip({ label, value }: MetricChipProps) {
  return (
    <Box sx={{ display: 'inline-flex', alignItems: 'center', gap: 0.5, mr: 1.5 }}>
      <Typography variant="caption" color="text.secondary">{label}</Typography>
      <Typography variant="caption" sx={{ fontWeight: 600 }}>{value}</Typography>
    </Box>
  )
}

export { MetricChip }
export type { MetricChipProps }
