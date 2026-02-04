import { Box, Typography } from '@mui/material'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'

type ChatMessageProps = {
  role: 'user' | 'assistant'
  content: string
  streaming?: boolean
}

function ChatMessage({ role, content, streaming }: ChatMessageProps) {
  if (role === 'user') {
    return (
      <Box
        sx={{
          py: 0.5,
          px: 1.5,
          bgcolor: 'action.hover',
          borderRadius: 1,
          alignSelf: 'flex-start',
          maxWidth: '80%',
        }}
      >
        <Typography
          variant="body2"
          sx={{
            whiteSpace: 'pre-wrap',
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
          }}
        >
          {content}
        </Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ py: 0.5 }}>
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
