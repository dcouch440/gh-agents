import { useMemo } from 'react'
import { Box } from '@mui/material'
import { TerminalThemeProvider } from './theme/TerminalThemeProvider'
import { parseMarkdown } from './parser/parse'
import { TextSpanRenderer } from './renderers/TextSpanRenderer'
import type { InlineNode } from './parser/types'

type TerminalInlineProps = {
  content: string
}

function TerminalInlineInner({ content }: TerminalInlineProps) {
  const inlineNodes = useMemo((): InlineNode[] => {
    const blocks = parseMarkdown(content)
    // Extract inline nodes from the first paragraph only
    const first = blocks[0]
    if (first?.type === 'paragraph') return first.children
    return [{ type: 'text', key: 'fallback-0', content }]
  }, [content])

  return (
    <Box
      component="span"
      sx={{
        display: 'inline',
        fontSize: 'inherit',
        lineHeight: 'inherit',
        color: 'inherit',
      }}
    >
      <TextSpanRenderer nodes={inlineNodes} />
    </Box>
  )
}

function TerminalInline(props: TerminalInlineProps) {
  return (
    <TerminalThemeProvider>
      <TerminalInlineInner {...props} />
    </TerminalThemeProvider>
  )
}

export { TerminalInline }
export type { TerminalInlineProps }
