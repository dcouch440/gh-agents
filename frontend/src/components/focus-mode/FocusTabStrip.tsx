import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { FOCUS_MODE } from '@/constants'
import type { CanvasFormTab } from '@/components/canvas/CanvasFormNode'

type FocusTabStripProps = {
  tabs: CanvasFormTab[]
  activeTabId: string
  onTabChange: (tabId: string) => void
  accentColor: string
}

function FocusTabStrip({ tabs, activeTabId, onTabChange, accentColor }: FocusTabStripProps) {
  const theme = useTheme()

  return (
    <Box
      role="tablist"
      sx={{
        height: FOCUS_MODE.TAB_STRIP_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        gap: 0.5,
        px: 2,
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
          <Box
            key={tab.id}
            data-testid={`focus-tab-${tab.id}`}
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
              display: 'flex',
              alignItems: 'center',
              gap: 0.75,
              px: 1.5,
              py: 0.75,
              borderRadius: '6px',
              cursor: 'pointer',
              position: 'relative',
              backgroundColor: 'transparent',
              transition: 'background-color 120ms ease',
              '&:hover': isActive ? {} : { backgroundColor: theme.palette.custom.hoverOverlay },
              ...(isActive
                ? {
                    '&::after': {
                      content: '""',
                      position: 'absolute',
                      bottom: -6,
                      left: 8,
                      right: 8,
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
                fontSize: 18,
                color: isActive ? accentColor : 'text.secondary',
                transition: 'color 120ms ease',
              }}
            />
            <Typography
              sx={{
                fontSize: 12,
                fontWeight: isActive ? 600 : 400,
                color: isActive ? 'text.primary' : 'text.secondary',
                lineHeight: 1,
                whiteSpace: 'nowrap',
                transition: 'color 120ms ease',
              }}
            >
              {tab.tooltip}
            </Typography>
          </Box>
        )
      })}
    </Box>
  )
}

export { FocusTabStrip }
export type { FocusTabStripProps }
