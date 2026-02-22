import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import { HeadingRenderer } from './HeadingRenderer'
import { ParagraphRenderer } from './ParagraphRenderer'
import { CodeBlockRenderer } from './CodeBlockRenderer'
import { BlockquoteRenderer } from './BlockquoteRenderer'
import { ListRenderer } from './ListRenderer'
import { TableRenderer } from './TableRenderer'
import { HorizontalRuleRenderer } from './HorizontalRuleRenderer'
import type { BlockNode } from '../parser/types'

type BlockRendererProps = {
  node: BlockNode
}

function BlockRenderer({ node }: BlockRendererProps) {
  const theme = useTerminalTheme()

  switch (node.type) {
    case 'heading':
      return <HeadingRenderer node={node} />

    case 'paragraph':
      return <ParagraphRenderer node={node} />

    case 'code_block':
      return <CodeBlockRenderer node={node} />

    case 'blockquote':
      return <BlockquoteRenderer node={node} />

    case 'list':
      return <ListRenderer node={node} />

    case 'list_item':
      // List items are rendered by ListRenderer — this shouldn't be reached directly
      return null

    case 'table':
      return <TableRenderer node={node} />

    case 'hr':
      return <HorizontalRuleRenderer />

    case 'html_block':
      return (
        <Box
          component="pre"
          sx={{
            m: 0,
            my: '0.4em',
            color: theme.textDisabled,
            fontFamily: 'inherit',
            fontSize: 'inherit',
            whiteSpace: 'pre-wrap',
          }}
        >
          {node.content}
        </Box>
      )
  }
}

export { BlockRenderer }
export type { BlockRendererProps }
