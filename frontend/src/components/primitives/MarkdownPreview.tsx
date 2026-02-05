import { Box } from '@mui/material'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import remarkBreaks from 'remark-breaks'

type MarkdownPreviewProps = {
  content: string
  className?: string
}

const stripThinkingTags = (text: string): string =>
  text.replace(/<thinking>[\s\S]*?<\/thinking>/g, '')

function MarkdownPreview({ content, className }: MarkdownPreviewProps) {
  const cleaned = stripThinkingTags(content)

  return (
    <Box
      className={className}
      sx={{
        overflow: 'auto',
        height: '100%',
        fontSize: '0.875rem',
        lineHeight: 1.6,
        color: 'text.primary',
        '& p': { mb: 0.5, '&:last-child': { mb: 0 } },
        '& code:not(pre code)': {
          color: 'primary.main',
          fontFamily: 'monospace',
          fontSize: '0.875em',
        },
        '& pre': { my: 0.5 },
        '& pre code': {
          display: 'block',
          borderLeft: 2,
          borderColor: 'divider',
          py: 1,
          px: 1.5,
          fontFamily: 'monospace',
          fontSize: '0.8125rem',
          whiteSpace: 'pre',
          overflowX: 'auto',
        },
        '& ul': { listStyleType: 'disc', listStylePosition: 'inside', mb: 0.5 },
        '& ol': { listStyleType: 'decimal', listStylePosition: 'inside', mb: 0.5 },
        '& table': { width: '100%', borderCollapse: 'collapse' },
        '& th': {
          fontWeight: 600,
          color: 'text.secondary',
          borderBottom: 1,
          borderColor: 'divider',
          py: 0.5,
          px: 1,
          textAlign: 'left',
        },
        '& td': {
          borderBottom: 1,
          borderColor: 'divider',
          py: 0.5,
          px: 1,
        },
      }}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm, remarkBreaks]}>
        {cleaned}
      </ReactMarkdown>
    </Box>
  )
}

export { MarkdownPreview }
export type { MarkdownPreviewProps }
