import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import { TextSpanRenderer } from './TextSpanRenderer'
import type { LinkNode } from '../parser/types'

type LinkRendererProps = {
  node: LinkNode
}

function LinkRenderer({ node }: LinkRendererProps) {
  const theme = useTerminalTheme()

  return (
    <Box
      component="a"
      href={node.href}
      target="_blank"
      rel="noopener noreferrer"
      sx={{
        color: theme.linkText,
        textDecoration: 'underline',
        textDecorationColor: theme.linkUnderline,
        textUnderlineOffset: '2px',
        '&:hover': { textDecorationColor: theme.linkText },
      }}
    >
      <TextSpanRenderer nodes={node.children} />
    </Box>
  )
}

export { LinkRenderer }
export type { LinkRendererProps }
