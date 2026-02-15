import Box from '@mui/material/Box'
import { Outlet, useLocation, useNavigate } from 'react-router-dom'
import { TopNavBar } from './TopNavBar'
import { ThemeToggle } from './ThemeToggle'
import { useStore, reviewQueueStore } from '@/stores'
import { useNavigation } from '@/hooks/useNavigation'
import { ROUTES } from '@/constants'
import type { NavBarItem } from './types'

function AppLayout() {
  const location = useLocation()
  const navigate = useNavigate()
  const { navItems, utilityItems } = useNavigation()
  const pendingCount = useStore(reviewQueueStore.store, reviewQueueStore.selectPendingCount)

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

  const isCanvasPage = location.pathname.startsWith(ROUTES.WORKFLOWS) && location.pathname !== ROUTES.WORKFLOWS


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

      </Box>
    </Box>
  )
}

export { AppLayout }
