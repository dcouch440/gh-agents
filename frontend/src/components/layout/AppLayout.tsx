import Box from '@mui/material/Box'
import { Outlet, Navigate, useLocation, useNavigate } from 'react-router-dom'
import { TopNavBar } from './TopNavBar'
import { ThemeToggle } from './ThemeToggle'
import { useStore, authStore, selectUser, selectAuthStatus, reviewQueueStore } from '@/stores'
import { useNavigation } from '@/hooks/useNavigation'
import { LoadingSpinner } from '@/components/primitives'
import { ROUTES } from '@/constants'
import type { NavBarItem } from './types'

function AppLayout() {
  const status = useStore(authStore.store, selectAuthStatus)
  const user = useStore(authStore.store, selectUser)
  const location = useLocation()
  const navigate = useNavigate()
  const { navItems, utilityItems } = useNavigation()
  const pendingCount = useStore(reviewQueueStore.store, reviewQueueStore.selectPendingCount)

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
