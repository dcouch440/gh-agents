import { Box, Typography } from '@mui/material'

type EmptyStateProps = {
  icon?: string
  message: string
}

function EmptyState({ icon, message }: EmptyStateProps) {
  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        p: 4,
        textAlign: 'center',
        gap: 1,
      }}
    >
      {icon ? (
        <Typography variant="h3" component="div" sx={{ opacity: 0.5 }}>
          {icon}
        </Typography>
      ) : null}
      <Typography variant="body2" color="text.secondary">
        {message}
      </Typography>
    </Box>
  )
}

export { EmptyState }
export type { EmptyStateProps }
