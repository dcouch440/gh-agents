import Box from '@mui/material/Box'
import Tooltip from '@mui/material/Tooltip'
import { useTheme } from '@mui/material/styles'
import { FORM_NODE } from './constants'
import type { CanvasFormTab } from './types'

type FormTabStripProps = {
  tabs: CanvasFormTab[]
  activeTabId: string
  onTabChange: (tabId: string) => void
  accentColor: string
}

function FormTabStrip({ tabs, activeTabId, onTabChange, accentColor }: FormTabStripProps) {
  const theme = useTheme()

  return (
    <Box
      className="nodrag"
      role="tablist"
      sx={{
        height: FORM_NODE.TAB_STRIP_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        gap: 0.25,
        px: 0.5,
        borderBottom: 1,
        borderColor: 'divider',
        backgroundColor: theme.palette.custom.bgHeader,
        flexShrink: 0,
      }}
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId
        const IconComponent = tab.icon
        return (
          <Tooltip key={tab.id} title={tab.tooltip} placement="bottom">
            <Box
              data-testid={`tab-${tab.id}`}
              onClick={() => {
                onTabChange(tab.id)
              }}
              role="tab"
              tabIndex={0}
              aria-selected={isActive}
              aria-label={tab.tooltip}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') onTabChange(tab.id)
              }}
              sx={{
                width: 28,
                height: 24,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: '4px',
                cursor: 'pointer',
                position: 'relative',
                backgroundColor: isActive ? theme.palette.custom.activeTint : 'transparent',
                transition: 'background-color 120ms ease',
                '&:hover': isActive ? {} : { backgroundColor: theme.palette.custom.hoverOverlay },
                ...(isActive
                  ? {
                      '&::after': {
                        content: '""',
                        position: 'absolute',
                        bottom: -4,
                        left: 4,
                        right: 4,
                        height: 2,
                        borderRadius: 1,
                        backgroundColor: accentColor,
                      },
                    }
                  : {}),
              }}
            >
              <IconComponent
                sx={{
                  fontSize: 16,
                  color: isActive ? accentColor : 'text.secondary',
                  transition: 'color 120ms ease',
                }}
              />
            </Box>
          </Tooltip>
        )
      })}
    </Box>
  )
}

export { FormTabStrip }
export type { FormTabStripProps }
