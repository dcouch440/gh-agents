import Box from '@mui/material/Box'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'
import { ToolIndicator } from './ToolIndicator'
import type { MessageSegment } from '@/types'

type StreamingMessageProps = {
  segments: MessageSegment[]
  streaming?: boolean
}

function StreamingMessage({ segments, streaming }: StreamingMessageProps) {
  const lastSegment = segments[segments.length - 1]
  const showCursor = streaming && lastSegment?.type === 'text' && lastSegment.content.length > 0

  return (
    <Box sx={{ py: 0.5 }}>
      {segments.map((segment, i) => {
        const key = `seg-${i}`

        switch (segment.type) {
          case 'text':
            return <MarkdownPreview key={key} content={segment.content} />
          case 'tool':
            return (
              <Box key={key} sx={{ display: 'block' }}>
                <ToolIndicator variant="tool" toolName={segment.toolName} status={segment.status} />
              </Box>
            )
          case 'doc_update':
            return (
              <Box key={key} sx={{ display: 'block' }}>
                <ToolIndicator variant="doc_update" title={segment.title} />
              </Box>
            )
        }
      })}
      {showCursor ? (
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

export { StreamingMessage }
export type { StreamingMessageProps }
