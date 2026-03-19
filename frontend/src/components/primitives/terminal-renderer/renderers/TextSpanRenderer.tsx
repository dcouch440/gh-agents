import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import type { InlineNode } from '../parser/types'

type TextSpanRendererProps = {
  nodes: InlineNode[]
}

function TextSpanRenderer({ nodes }: TextSpanRendererProps) {
  const theme = useTerminalTheme()

  return (
    <>
      {nodes.map((node) => {
        switch (node.type) {
          case 'text':
            return <span key={node.key}>{node.content}</span>

          case 'strong':
            return (
              <Box key={node.key} component="span" sx={{ color: theme.bright }}>
                <TextSpanRenderer nodes={node.children} />
              </Box>
            )

          case 'emphasis':
            return (
              <Box key={node.key} component="span" sx={{ color: theme.dim }}>
                <TextSpanRenderer nodes={node.children} />
              </Box>
            )

          case 'strikethrough':
            return (
              <Box
                key={node.key}
                component="span"
                sx={{ color: theme.dimStrike, textDecoration: 'line-through' }}
              >
                <TextSpanRenderer nodes={node.children} />
              </Box>
            )

          case 'inline_code':
            return (
              <Box
                key={node.key}
                component="code"
                sx={{
                  color: theme.codeText,
                  bgcolor: theme.codeBg,
                  fontFamily: '"JetBrains Mono", monospace',
                  px: '0.3em',
                  py: '0.1em',
                  borderRadius: '2px',
                  fontSize: '0.9em',
                }}
              >
                {node.content}
              </Box>
            )

          case 'link':
            return (
              <Box
                key={node.key}
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

          case 'image':
            return (
              <Box key={node.key} component="span" sx={{ color: theme.dim }}>
                [img: {node.alt || 'image'}]
              </Box>
            )

          case 'softbreak':
            return <span key={node.key}> </span>

          case 'hardbreak':
            return <br key={node.key} />
        }
      })}
    </>
  )
}

export { TextSpanRenderer }
export type { TextSpanRendererProps }
