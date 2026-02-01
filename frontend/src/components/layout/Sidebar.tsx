import { NavLink } from 'react-router-dom'
import { APP_NAME, ROUTES } from '@/constants'

const NAV_ITEMS = [
  { to: ROUTES.DASHBOARD, label: 'Dashboard' },
  { to: ROUTES.CHAT, label: 'Chat' },
  { to: ROUTES.AGENTS, label: 'Agents' },
  { to: ROUTES.PIPELINES, label: 'Pipelines' },
  { to: ROUTES.TASKS, label: 'Tasks' },
  { to: ROUTES.DOCUMENTS, label: 'Documents' },
  { to: ROUTES.SETTINGS, label: 'Settings' },
  { to: ROUTES.SHOWCASE, label: 'Showcase' },
] as const

function Sidebar() {
  return (
    <nav style={{
      width: 220,
      borderRight: '1px solid #e2e2e2',
      padding: '1rem',
      display: 'flex',
      flexDirection: 'column',
      gap: '0.25rem',
    }}>
      <div style={{ fontWeight: 700, fontSize: '1.25rem', marginBottom: '1rem' }}>
        {APP_NAME}
      </div>
      {NAV_ITEMS.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.to === '/'}
          style={({ isActive }) => ({
            display: 'block',
            padding: '0.5rem 0.75rem',
            borderRadius: 6,
            textDecoration: 'none',
            color: isActive ? '#fff' : '#333',
            background: isActive ? '#333' : 'transparent',
          })}
        >
          {item.label}
        </NavLink>
      ))}
    </nav>
  )
}

export { Sidebar }
