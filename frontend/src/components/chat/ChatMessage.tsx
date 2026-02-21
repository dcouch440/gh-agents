import { Box, Typography } from '@mui/material'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'

type ChatMessageProps = {
  role: 'user' | 'assistant' | 'system'
  content: string
  streaming?: boolean
}

function ChatMessage({ role, content, streaming }: ChatMessageProps) {
  if (role === 'system') {
    return (
      <Box
        sx={{
          py: 1,
          px: 1.5,
          mx: -1.5,
          mb: 1,
          borderLeft: 2,
          borderColor: 'primary.main',
          bgcolor: (theme) =>
            theme.palette.mode === 'light' ? 'rgba(99, 102, 241, 0.06)' : 'rgba(99, 102, 241, 0.10)',
        }}
      >
        <Typography
          variant="caption"
          sx={{
            fontWeight: 600,
            mb: 0.5,
            display: 'block',
            color: 'primary.main',
            letterSpacing: '0.05em',
          }}
        >
          SYSTEM PROMPT
        </Typography>
        <Typography
          component="pre"
          sx={{
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
            lineHeight: 1.5,
            color: 'text.secondary',
            m: 0,
          }}
        >
          {content}
        </Typography>
      </Box>
    )
  }

  if (role === 'user') {
    return (
      <Box
        sx={{
          py: 0.25,
          px: 1.5,
          mx: -1.5,
          bgcolor: (theme) =>
            theme.palette.mode === 'light' ? 'rgba(90, 138, 110, 0.08)' : 'rgba(255, 255, 255, 0.03)',
          boxShadow: (theme) =>
            theme.palette.mode === 'light'
              ? 'inset 0 1px 2px rgba(90, 138, 110, 0.06), inset 0 -1px 1px rgba(90, 138, 110, 0.04)'
              : 'inset 0 1px 2px rgba(0, 0, 0, 0.12), inset 0 -1px 1px rgba(0, 0, 0, 0.08)',
        }}
      >
        <Typography
          variant="body2"
          sx={{
            whiteSpace: 'pre-wrap',
            fontFamily: 'monospace',
            fontSize: '0.875rem',
          }}
        >
          {content}
        </Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ py: 0.25 }}>
      <MarkdownPreview content={content} />
      {streaming && content ? (
        <Box
          component="span"
          sx={{
            display: 'inline-block',
            width: '2px',
            height: '1.1em',
            bgcolor: 'primary.main',
            ml: 0.25,
            verticalAlign: 'text-bottom',
            animation: 'blink 1s cubic-bezier(0.4, 0, 0.6, 1) infinite',
            '@keyframes blink': {
              '0%, 100%': { opacity: 1 },
              '50%': { opacity: 0 },
            },
          }}
        />
      ) : null}
    </Box>
  )
}

export { ChatMessage }
export type { ChatMessageProps }
