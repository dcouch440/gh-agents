import { Box, Button, Typography } from '@mui/material'

export type MessageErrorNoticeProps = {
  error: string
  onRetry?: (() => void) | null
}

/**
 * Durable failure notice for a chat turn that never produced a reply.
 *
 * The live SSE error chunk only reaches a client that is attached at the
 * moment of failure, so this renders the failure recorded on the message
 * itself — which survives a reload or a dropped connection.
 */
function MessageErrorNotice({ error, onRetry }: MessageErrorNoticeProps) {
  return (
    <Box
      sx={{
        py: 1,
        px: 1.5,
        mx: -1.5,
        mb: 1,
        borderLeft: 2,
        borderColor: 'error.main',
        bgcolor: (theme) =>
          theme.palette.mode === 'light' ? 'rgba(211, 47, 47, 0.06)' : 'rgba(211, 47, 47, 0.12)',
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'space-between',
        gap: 1.5,
      }}
    >
      <Box sx={{ minWidth: 0 }}>
        <Typography
          variant="caption"
          sx={{
            fontWeight: 600,
            mb: 0.5,
            display: 'block',
            color: 'error.main',
            letterSpacing: '0.05em',
          }}
        >
          NO RESPONSE
        </Typography>
        <Typography
          variant="body2"
          sx={{
            color: 'text.secondary',
            wordBreak: 'break-word',
            fontSize: '0.8125rem',
          }}
        >
          {error}
        </Typography>
      </Box>
      {onRetry ? (
        <Button size="small" color="error" variant="outlined" onClick={onRetry} sx={{ flexShrink: 0 }}>
          Retry
        </Button>
      ) : null}
    </Box>
  )
}

export { MessageErrorNotice }
