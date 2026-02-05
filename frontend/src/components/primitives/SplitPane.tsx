import type { ReactNode } from 'react'
import { Box } from '@mui/material'

type SplitPaneProps = {
  left: ReactNode
  right: ReactNode
  splitPercent: number
  onMouseDown: (e: React.MouseEvent) => void
  className?: string
}

function SplitPane({ left, right, splitPercent, onMouseDown, className }: SplitPaneProps) {
  return (
    <Box
      className={className}
      sx={{
        display: 'flex',
        height: '100%',
        overflow: 'hidden',
      }}
    >
      <Box
        sx={{
          width: `${splitPercent}%`,
          overflow: 'auto',
          minWidth: 0,
        }}
      >
        {left}
      </Box>
      <Box
        onMouseDown={onMouseDown}
        sx={{
          flexShrink: 0,
          width: 6,
          cursor: 'col-resize',
          bgcolor: 'divider',
          transition: 'background-color 150ms ease',
          '&:hover, &:active': {
            bgcolor: 'primary.main',
          },
        }}
      />
      <Box
        sx={{
          flex: 1,
          overflow: 'auto',
          minWidth: 0,
        }}
      >
        {right}
      </Box>
    </Box>
  )
}

export { SplitPane }
export type { SplitPaneProps }
