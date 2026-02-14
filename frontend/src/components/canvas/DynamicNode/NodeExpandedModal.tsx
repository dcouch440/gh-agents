import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import Dialog from '@mui/material/Dialog'
import IconButton from '@mui/material/IconButton'
import CloseOutlined from '@mui/icons-material/CloseOutlined'
import { useTheme } from '@mui/material/styles'
import { LAYOUT, ANIMATION } from '@/constants'
import { FormTabStrip } from '../CanvasFormNode'
import type { CanvasFormTab } from '../CanvasFormNode'

type NodeExpandedModalProps = {
  open: boolean
  onClose: () => void
  header: ReactNode
  tabs: CanvasFormTab[]
  activeTabId: string
  onTabChange: (tabId: string) => void
  accentColor: string
}

function NodeExpandedModal({
  open,
  onClose,
  header,
  tabs,
  activeTabId,
  onTabChange,
  accentColor,
}: NodeExpandedModalProps) {
  const theme = useTheme()
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0]

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth={false}
      transitionDuration={ANIMATION.NORMAL}
      sx={{
        top: `${LAYOUT.TOPBAR_HEIGHT}px`,
        '& .MuiDialog-paper': {
          m: 0,
          maxHeight: 'none',
          maxWidth: 'none',
          width: '100%',
          height: '100%',
          borderRadius: 0,
          border: 'none',
          borderTop: `1px solid ${theme.palette.divider}`,
          display: 'flex',
          flexDirection: 'column',
        },
        '& .MuiBackdrop-root': {
          backdropFilter: 'blur(4px)',
        },
      }}
    >
      {/* Header */}
      <Box
        sx={{
          height: 60,
          display: 'flex',
          alignItems: 'center',
          borderBottom: 1,
          borderColor: 'divider',
          backgroundColor: theme.palette.custom.bgHeader,
          flexShrink: 0,
        }}
      >
        <Box sx={{ flex: 1, height: '100%' }}>{header}</Box>
        <IconButton
          onClick={onClose}
          size="small"
          sx={{
            flexShrink: 0,
            mr: 1.5,
            width: 32,
            height: 32,
            color: 'text.secondary',
            '&:hover': { color: 'text.primary' },
          }}
        >
          <CloseOutlined sx={{ fontSize: 20 }} />
        </IconButton>
      </Box>

      {/* Tab strip */}
      <FormTabStrip
        tabs={tabs}
        activeTabId={activeTabId}
        onTabChange={onTabChange}
        accentColor={accentColor}
      />

      {/* Content area */}
      <Box
        sx={{
          flex: 1,
          minHeight: 0,
          overflow: 'hidden',
          position: 'relative',
          cursor: 'text',
          userSelect: 'text',
        }}
      >
        {activeTab?.content}
      </Box>
    </Dialog>
  )
}

export { NodeExpandedModal }
export type { NodeExpandedModalProps }
