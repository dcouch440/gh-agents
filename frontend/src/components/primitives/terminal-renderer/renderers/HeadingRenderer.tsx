import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import { TextSpanRenderer } from './TextSpanRenderer'
import type { HeadingNode } from '../parser/types'

type HeadingRendererProps = {
  node: HeadingNode
}

function HeadingRenderer({ node }: HeadingRendererProps) {
  const theme = useTerminalTheme()

  switch (node.level) {
    // ═══ HEADING TEXT ═══
    case 1:
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: '0.6em', my: '0.6em', color: theme.headingText }}>
          <Box component="span" sx={{ color: theme.headingRule, flexShrink: 0, whiteSpace: 'pre' }}>{'═══'}</Box>
          <Box component="span" sx={{ textTransform: 'uppercase', whiteSpace: 'nowrap' }}>
            <TextSpanRenderer nodes={node.children} />
          </Box>
          <Box sx={{ flex: 1, height: '1px', minWidth: '2em' }}>
            <Box sx={{ borderTop: `2px double ${theme.headingRule}`, width: '100%' }} />
          </Box>
        </Box>
      )

    // ─── HEADING TEXT ───
    case 2:
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: '0.6em', my: '0.5em', color: theme.headingText }}>
          <Box component="span" sx={{ color: theme.headingRule, flexShrink: 0, whiteSpace: 'pre' }}>{'───'}</Box>
          <Box component="span" sx={{ textTransform: 'uppercase', whiteSpace: 'nowrap' }}>
            <TextSpanRenderer nodes={node.children} />
          </Box>
          <Box sx={{ flex: 1, height: '1px', minWidth: '2em' }}>
            <Box sx={{ borderTop: `1px solid ${theme.headingRule}`, width: '100%' }} />
          </Box>
        </Box>
      )

    // ── Heading Text
    case 3:
      return (
        <Box sx={{ my: '0.4em', color: theme.headingText }}>
          <Box component="span" sx={{ color: theme.headingRule, whiteSpace: 'pre' }}>{'── '}</Box>
          <TextSpanRenderer nodes={node.children} />
        </Box>
      )

    // ▸ Heading Text
    case 4:
      return (
        <Box sx={{ my: '0.3em', color: theme.headingText }}>
          <Box component="span" sx={{ color: theme.headingRule, mr: '0.4em' }}>{'▸'}</Box>
          <TextSpanRenderer nodes={node.children} />
        </Box>
      )

    // Bright color only
    case 5:
      return (
        <Box sx={{ my: '0.3em', color: theme.headingText }}>
          <TextSpanRenderer nodes={node.children} />
        </Box>
      )

    // Secondary color only
    case 6:
      return (
        <Box sx={{ my: '0.2em', color: theme.textSecondary }}>
          <TextSpanRenderer nodes={node.children} />
        </Box>
      )
  }
}

export { HeadingRenderer }
export type { HeadingRendererProps }
