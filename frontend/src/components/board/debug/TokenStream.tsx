import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type TokenStreamProps = {
  readonly text: string
}

/**
 * Monospace box that displays a streaming token buffer.
 * Renders as a scrollable pre-formatted block.
 */
function TokenStream({ text }: TokenStreamProps) {
  if (text.length === 0) return null

  return (
    <Box
      sx={{
        bgcolor: 'background.default',
        borderRadius: 1,
        p: 1,
        maxHeight: 200,
        overflowY: 'auto',
        border: 1,
        borderColor: 'divider',
      }}
    >
      <Typography
        component="pre"
        variant="caption"
        sx={{
          fontFamily: 'monospace',
          fontSize: 11,
          lineHeight: 1.5,
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
          m: 0,
          color: 'text.primary',
        }}
      >
        {text}
      </Typography>
    </Box>
  )
}

export { TokenStream }
export type { TokenStreamProps }
