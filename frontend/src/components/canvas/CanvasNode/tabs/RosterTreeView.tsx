import { useMemo } from 'react'
import Box from '@mui/material/Box'
import { useStore, workflowStore } from '@/stores'
import { AsciiTree } from '@/utils/AsciiTree'
import type { RosterAgent } from '@/types/workflow'

type RosterTreeViewProps = {
  stepId: string
}

function RosterTreeView({ stepId }: RosterTreeViewProps) {
  const roster = useStore(workflowStore.store, workflowStore.selectStepRoster(stepId))

  const rendered = useMemo(() => {
    if (roster.length === 0) return null

    const tree = AsciiTree.from<RosterAgent>(roster, {
      id: (a) => a.id,
      label: (a) => a.name,
      parentId: (a) => a.depends_on[0] ?? null,
      detail: (a) => a.capabilities.length > 0 ? a.capabilities.join(', ') : null,
      sortBy: (a, b) => a.execution_order - b.execution_order,
    })

    if (tree.isEmpty) return null

    return tree.render()
  }, [roster])

  if (!rendered) return null

  return (
    <Box sx={{ px: 1.5, pt: 1.5 }}>
      <Box
        component="pre"
        sx={{
          m: 0,
          py: '0.5em',
          pl: '1em',
          fontFamily: '"JetBrains Mono", monospace',
          fontSize: '0.8125rem',
          lineHeight: 1.5,
          color: 'text.secondary',
          whiteSpace: 'pre',
          borderLeft: 2,
          borderColor: 'divider',
          bgcolor: (theme) =>
            theme.palette.mode === 'dark'
              ? 'rgba(255,255,255,0.04)'
              : 'rgba(0,0,0,0.04)',
        }}
      >
        {rendered}
      </Box>
    </Box>
  )
}

export { RosterTreeView }
export type { RosterTreeViewProps }
