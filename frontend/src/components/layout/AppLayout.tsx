import { createElement } from 'react'
import Box from '@mui/material/Box'
import { Outlet, Navigate, useLocation, useNavigate } from 'react-router-dom'
import TuneOutlined from '@mui/icons-material/TuneOutlined'
import EditNoteOutlined from '@mui/icons-material/EditNoteOutlined'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import DataObjectOutlined from '@mui/icons-material/DataObjectOutlined'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import { TopNavBar } from './TopNavBar'
import { IconRail } from './IconRail'
import { DetailPanel } from './DetailPanel'
import { ThemeToggle } from './ThemeToggle'
import { useStore, authStore, selectUser, selectAuthStatus, layoutStore, reviewQueueStore } from '@/stores'
import { useNavigation } from '@/hooks/useNavigation'
import { LoadingSpinner } from '@/components/primitives'
import { RightPanelContent } from '@/components/panels'
import { ROUTES } from '@/constants'
import type { NavBarItem } from './types'
import type { RailItem } from './IconRail'

function AppLayout() {
  const status = useStore(authStore.store, selectAuthStatus)
  const user = useStore(authStore.store, selectUser)
  const location = useLocation()
  const navigate = useNavigate()
  const { navItems, utilityItems } = useNavigation()
  const pendingCount = useStore(reviewQueueStore.store, reviewQueueStore.selectPendingCount)
  const rightOpen = useStore(layoutStore.store, layoutStore.selectRightPanelOpen)
  const rightSection = useStore(layoutStore.store, layoutStore.selectRightPanelSection)
  const rightWidth = useStore(layoutStore.store, layoutStore.selectRightPanelWidth)
  const rightDragging = useStore(layoutStore.store, layoutStore.selectRightPanelDragging)

  if (status === 'idle' || status === 'loading') {
    return (
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '100vh',
        }}
      >
        <LoadingSpinner label="Loading..." />
      </Box>
    )
  }

  if (!user) {
    return <Navigate to={ROUTES.LOGIN} state={{ from: location }} replace />
  }

  const toNavItem = (item: { path: string; icon: React.ReactNode; label: string; isActive: boolean }): NavBarItem => ({
    key: item.path,
    icon: item.icon,
    label: item.label,
    isActive: item.isActive,
    badge: item.path === ROUTES.REVIEW_QUEUE ? pendingCount : undefined,
    onClick: () => {
      void navigate(item.path)
    },
  })

  const topNavItems = navItems.map(toNavItem)
  const utilNavItems = utilityItems.map(toNavItem)

  const showRightRail = location.pathname.startsWith(ROUTES.WORKFLOWS)
  const isCanvasPage = showRightRail && location.pathname !== ROUTES.WORKFLOWS

  const rightRailItems: RailItem[] = [
    {
      key: 'properties',
      icon: createElement(TuneOutlined, { fontSize: 'small' }),
      label: 'Properties',
      isActive: rightOpen && rightSection === 'properties',
      onClick: () => {
        layoutStore.toggleRightPanel('properties')
      },
    },
    {
      key: 'prompts',
      icon: createElement(EditNoteOutlined, { fontSize: 'small' }),
      label: 'Prompts',
      isActive: rightOpen && rightSection === 'prompts',
      onClick: () => {
        layoutStore.toggleRightPanel('prompts')
      },
    },
    {
      key: 'agents',
      icon: createElement(SmartToyOutlined, { fontSize: 'small' }),
      label: 'Agents',
      isActive: rightOpen && rightSection === 'agents',
      onClick: () => {
        layoutStore.toggleRightPanel('agents')
      },
    },
    {
      key: 'schemas',
      icon: createElement(DataObjectOutlined, { fontSize: 'small' }),
      label: 'Schemas',
      isActive: rightOpen && rightSection === 'schemas',
      onClick: () => {
        layoutStore.toggleRightPanel('schemas')
      },
    },
    {
      key: 'execution',
      icon: createElement(PlayArrowOutlined, { fontSize: 'small' }),
      label: 'Execution',
      isActive: rightOpen && rightSection === 'execution',
      onClick: () => {
        layoutStore.toggleRightPanel('execution')
      },
    },
  ]

  const rightPanelTitle = rightRailItems.find((i) => i.key === rightSection)?.label ?? ''

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      {/* Top: horizontal navigation bar */}
      <TopNavBar navItems={topNavItems} utilityItems={utilNavItems} trailing={<ThemeToggle />} />

      {/* Body: main content + optional right sidebar */}
      <Box sx={{ display: 'flex', flexGrow: 1, minHeight: 0 }}>
        {/* Main content */}
        <Box
          component="main"
          sx={{
            flexGrow: 1,
            overflow: isCanvasPage ? 'hidden' : 'auto',
            px: isCanvasPage ? 0 : 2.5,
            py: isCanvasPage ? 0 : 2,
            minWidth: 0,
            backgroundColor: (theme) => theme.palette.custom.cavityBg,
          }}
        >
          <Outlet />
        </Box>

        {/* Right detail panel + rail — workflow pages only */}
        {showRightRail && (
          <>
            <DetailPanel
              side="right"
              isOpen={rightOpen}
              onClose={layoutStore.closeRightPanel}
              title={rightPanelTitle}
              width={rightWidth}
              isDragging={rightDragging}
              onResize={layoutStore.setRightPanelWidth}
              onDragStart={layoutStore.startRightPanelDrag}
              onDragEnd={layoutStore.stopRightPanelDrag}
            >
              <RightPanelContent section={rightSection} />
            </DetailPanel>
            <IconRail side="right" topItems={rightRailItems} />
          </>
        )}
      </Box>
    </Box>
  )
}

export { AppLayout }
