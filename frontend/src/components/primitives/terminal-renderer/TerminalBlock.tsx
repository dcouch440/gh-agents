import { memo, useMemo } from 'react'
import { Box } from '@mui/material'
import { TerminalThemeProvider } from './theme/TerminalThemeProvider'
import { useStripThinking } from './hooks/useStripThinking'
import { parseMarkdown } from './parser/parse'
import { BlockRenderer } from './renderers/BlockRenderer'

type TerminalBlockProps = {
  content: string
  className?: string
}

const MemoizedBlockRenderer = memo(BlockRenderer)

function TerminalBlockInner({ content, className }: TerminalBlockProps) {
  const cleaned = useStripThinking(content)
  const blocks = useMemo(() => parseMarkdown(cleaned), [cleaned])

  return (
    <Box
      className={className}
      sx={{
        overflow: 'auto',
        height: '100%',
        contain: 'layout style paint',
        fontFamily: 'inherit',
        fontSize: '0.875rem',
        fontWeight: 500,
        lineHeight: 1.6,
        color: 'text.primary',
        fontVariantLigatures: 'none',
        fontFeatureSettings: '"liga" 0, "calt" 0',
        WebkitFontSmoothing: 'antialiased',
        MozOsxFontSmoothing: 'grayscale',
      }}
    >
      {blocks.map((node) => (
        <Box
          key={node.key}
          sx={{
            contentVisibility: 'auto',
            containIntrinsicSize: 'auto 1.5em',
          }}
        >
          <MemoizedBlockRenderer node={node} />
        </Box>
      ))}
    </Box>
  )
}

const TerminalBlock = memo(function TerminalBlock(props: TerminalBlockProps) {
  return (
    <TerminalThemeProvider>
      <TerminalBlockInner {...props} />
    </TerminalThemeProvider>
  )
})

export { TerminalBlock }
export type { TerminalBlockProps }
