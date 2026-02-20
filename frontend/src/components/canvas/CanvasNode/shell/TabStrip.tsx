import Box from '@mui/material/Box'
import Tooltip from '@mui/material/Tooltip'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { FORM_NODE } from '../../CanvasFormNode/constants'
import { FOCUS_MODE } from '@/constants'
import type { CanvasFormTab } from '../../CanvasFormNode/types'

type TabStripVariant = 'compact' | 'full'

type TabStripProps = {
  tabs: CanvasFormTab[]
  activeTabId: string
  onTabChange: (tabId: string) => void
  accentColor: string
  variant?: TabStripVariant
}

function TabStrip({ tabs, activeTabId, onTabChange, accentColor, variant = 'compact' }: TabStripProps) {
  const theme = useTheme()
  const isCompact = variant === 'compact'
  const activeTab = tabs.find((t) => t.id === activeTabId)

  return (
    <Box
      className={isCompact ? 'nodrag' : undefined}
      role="tablist"
      sx={{
        height: isCompact ? FORM_NODE.TAB_STRIP_HEIGHT : FOCUS_MODE.TAB_STRIP_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        gap: isCompact ? 0.25 : 0.5,
        px: isCompact ? 0.5 : 2,
        backgroundColor: isCompact ? 'transparent' : theme.palette.custom.bgHeader,
        ...(isCompact ? {} : { borderBottom: 1, borderColor: 'divider' }),
        flexShrink: 0,
      }}
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId
        const IconComponent = tab.icon
        return isCompact ? (
          <Tooltip key={tab.id} title={tab.tooltip} placement="bottom">
            <Box
              data-testid={`tab-${tab.id}`}
              onClick={() => { onTabChange(tab.id) }}
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
                borderRadius: '5px',
                cursor: 'pointer',
                backgroundColor: isActive ? theme.palette.custom.accentBg : 'transparent',
                transition: 'background-color 120ms ease, color 120ms ease',
                '&:hover': isActive ? {} : { backgroundColor: theme.palette.custom.accentBg },
              }}
            >
              <IconComponent
                sx={{
                  fontSize: 16,
                  color: isActive ? theme.palette.custom.accent : theme.palette.text.disabled,
                  transition: 'color 120ms ease',
                }}
              />
            </Box>
          </Tooltip>
        ) : (
          <Box
            key={tab.id}
            data-testid={`tab-${tab.id}`}
            onClick={() => { onTabChange(tab.id) }}
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
      {isCompact && activeTab?.actions !== undefined && (
        <Box sx={{ marginLeft: 'auto', display: 'flex', alignItems: 'center' }}>
          {activeTab.actions}
        </Box>
      )}
    </Box>
  )
}

export { TabStrip }
export type { TabStripProps, TabStripVariant }
