import { Outlet } from 'react-router-dom'
import { Sidebar } from './Sidebar'

function AppLayout() {
  return (
    <div style={{ display: 'flex', height: '100vh' }}>
      <Sidebar />
      <main style={{ flex: 1, overflow: 'auto', padding: '1.5rem' }}>
        <Outlet />
      </main>
    </div>
  )
}

export { AppLayout }
