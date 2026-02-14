import { Box } from '@mui/material'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

type InlineMarkdownProps = {
  content: string
}

function InlineMarkdown({ content }: InlineMarkdownProps) {
  return (
    <Box
      component="span"
      sx={{
        display: 'inline',
        fontSize: 'inherit',
        lineHeight: 'inherit',
        color: 'inherit',
        '& p': { display: 'inline', m: 0 },
        '& code': {
          color: 'primary.main',
          fontFamily: 'monospace',
          fontSize: '0.875em',
        },
      }}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{content}</ReactMarkdown>
    </Box>
  )
}

export { InlineMarkdown }
export type { InlineMarkdownProps }
