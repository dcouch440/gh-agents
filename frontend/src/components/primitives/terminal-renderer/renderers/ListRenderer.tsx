import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import { BlockRenderer } from './BlockRenderer'
import type { ListNode, ListItemNode } from '../parser/types'

type ListRendererProps = {
  node: ListNode
  depth?: number
}

const BULLETS = ['▸', '◦', '·'] as const

const getBullet = (depth: number): string => BULLETS[depth % BULLETS.length]!

function ListItemRenderer({ item, ordered, index, depth }: {
  item: ListItemNode
  ordered: boolean
  index: number
  depth: number
}) {
  const theme = useTerminalTheme()

  const marker = (() => {
    if (item.taskChecked !== null) {
      const checked = item.taskChecked
      return (
        <Box
          component="span"
          sx={{
            color: checked ? theme.checkboxChecked : theme.checkboxUnchecked,
            mr: '0.5em',
            flexShrink: 0,
          }}
        >
          {checked ? '[x]' : '[ ]'}
        </Box>
      )
    }

    if (ordered) {
      return (
        <Box component="span" sx={{ color: theme.textSecondary, mr: '0.5em', flexShrink: 0 }}>
          {index}.
        </Box>
      )
    }

    return (
      <Box component="span" sx={{ color: theme.textSecondary, mr: '0.5em', flexShrink: 0 }}>
        {getBullet(depth)}
      </Box>
    )
  })()

  return (
    <Box sx={{ display: 'flex', alignItems: 'baseline', mb: '0.15em' }}>
      {marker}
      <Box sx={{ flex: 1, minWidth: 0 }}>
        {item.children.map((child) => {
          if (child.type === 'list') {
            return <ListRenderer key={child.key} node={child} depth={depth + 1} />
          }
          return <BlockRenderer key={child.key} node={child} />
        })}
      </Box>
    </Box>
  )
}

function ListRenderer({ node, depth = 0 }: ListRendererProps) {
  return (
    <Box sx={{ my: '0.3em', pl: depth > 0 ? '1.2em' : 0 }}>
      {node.children.map((item, i) => (
        <ListItemRenderer
          key={item.key}
          item={item}
          ordered={node.ordered}
          index={node.start + i}
          depth={depth}
        />
      ))}
    </Box>
  )
}

export { ListRenderer }
export type { ListRendererProps }
