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

function FormTabStrip({ tabs, activeTabId, onTabChange }: FormTabStripProps) {
  const theme = useTheme()
  const activeTab = tabs.find((t) => t.id === activeTabId)

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
        backgroundColor: 'transparent',
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
        )
      })}
      {activeTab?.actions !== undefined && (
        <Box sx={{ marginLeft: 'auto', display: 'flex', alignItems: 'center' }}>
          {activeTab.actions}
        </Box>
      )}
    </Box>
  )
}

export { FormTabStrip }
export type { FormTabStripProps }
