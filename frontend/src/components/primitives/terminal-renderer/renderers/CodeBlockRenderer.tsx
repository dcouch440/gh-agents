import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import type { CodeBlockNode } from '../parser/types'

type CodeBlockRendererProps = {
  node: CodeBlockNode
}

function CodeBlockRenderer({ node }: CodeBlockRendererProps) {
  const theme = useTerminalTheme()

  // Strip trailing newline that markdown-it adds
  const content = node.content.endsWith('\n') ? node.content.slice(0, -1) : node.content

  return (
    <Box sx={{ my: '0.4em' }}>
      {node.language ? (
        <Box
          component="span"
          sx={{
            color: theme.textDisabled,
            fontSize: '0.85em',
            display: 'block',
            mb: '0.15em',
          }}
        >
          [{node.language}]
        </Box>
      ) : null}
      <Box
        component="pre"
        sx={{
          m: 0,
          py: '0.5em',
          pl: '1em',
          borderLeft: `2px solid ${theme.codeBorder}`,
          bgcolor: theme.codeBg,
          overflowX: 'auto',
        }}
      >
        <Box component="code" sx={{ fontSize: 'inherit', fontFamily: 'inherit', whiteSpace: 'pre' }}>
          {content}
        </Box>
      </Box>
    </Box>
  )
}

export { CodeBlockRenderer }
export type { CodeBlockRendererProps }
