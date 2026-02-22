import { Box } from '@mui/material'
import { TextSpanRenderer } from './TextSpanRenderer'
import type { ParagraphNode } from '../parser/types'

type ParagraphRendererProps = {
  node: ParagraphNode
}

function ParagraphRenderer({ node }: ParagraphRendererProps) {
  return (
    <Box
      component="p"
      sx={{
        m: 0,
        mb: '0.4em',
        wordBreak: 'break-word',
        '&:last-child': { mb: 0 },
      }}
    >
      <TextSpanRenderer nodes={node.children} />
    </Box>
  )
}

export { ParagraphRenderer }
export type { ParagraphRendererProps }
