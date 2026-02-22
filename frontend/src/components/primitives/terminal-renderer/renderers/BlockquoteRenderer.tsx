import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import type { BlockquoteNode } from '../parser/types'

// Forward declaration — BlockRenderer is imported dynamically to avoid circular deps
import { BlockRenderer } from './BlockRenderer'

type BlockquoteRendererProps = {
  node: BlockquoteNode
}

function BlockquoteRenderer({ node }: BlockquoteRendererProps) {
  const theme = useTerminalTheme()

  return (
    <Box
      sx={{
        borderLeft: `2px solid ${theme.quoteBorder}`,
        pl: '1em',
        my: '0.4em',
        color: theme.quoteText,
      }}
    >
      {node.children.map((child) => (
        <BlockRenderer key={child.key} node={child} />
      ))}
    </Box>
  )
}

export { BlockquoteRenderer }
export type { BlockquoteRendererProps }
