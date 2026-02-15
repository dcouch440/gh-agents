import { Outlet, Navigate, useLocation } from 'react-router-dom'
import Box from '@mui/material/Box'
import { useStore, authStore, selectUser, selectAuthStatus } from '@/stores'
import { LoadingSpinner } from '@/components/primitives'
import { ROUTES } from '@/constants'

function AuthGuard() {
  const status = useStore(authStore.store, selectAuthStatus)
  const user = useStore(authStore.store, selectUser)
  const location = useLocation()

  if (status === 'idle' || status === 'loading') {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh' }}>
        <LoadingSpinner label="Loading..." />
      </Box>
    )
  }

  if (!user) {
    return <Navigate to={ROUTES.LOGIN} state={{ from: location }} replace />
  }

  return <Outlet />
}

export { AuthGuard }
