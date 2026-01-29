import { NavLink, useNavigate } from 'react-router-dom';
import {
  MessageSquare,
  Activity,
  ListTodo,
  Users,
  FolderOpen,
  BarChart3,
  Settings,
  LogOut,
} from 'lucide-react';
import { useAuthStore } from '../../store';

interface SidebarProps {
  collapsed: boolean;
}

const navItems = [
  { to: '/chat', icon: MessageSquare, label: 'Chat' },
  { to: '/feed', icon: Activity, label: 'Feed' },
  { to: '/tasks', icon: ListTodo, label: 'Tasks' },
  { to: '/agents', icon: Users, label: 'Agents' },
  { to: '/files', icon: FolderOpen, label: 'Files' },
  { to: '/stats', icon: BarChart3, label: 'Stats' },
];

export function Sidebar({ collapsed }: SidebarProps) {
  const navigate = useNavigate();
  const logout = useAuthStore((state) => state.logout);

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  return (
    <aside
      className={`bg-bg-secondary border-r border-border flex flex-col
                  transition-all duration-200
                  fixed lg:relative inset-y-0 left-0 z-40
                  ${collapsed ? '-translate-x-full lg:translate-x-0 lg:w-16' : 'w-56'}`}
    >
      {/* Logo */}
      <div className="h-14 flex items-center px-4 border-b border-border">
        <span className="text-xl font-bold text-text-primary font-mono">
          {collapsed ? 'n' : 'nexor'}
        </span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 py-4">
        <ul className="space-y-1 px-2">
          {navItems.map(({ to, icon: Icon, label }) => (
            <li key={to}>
              <NavLink
                to={to}
                className={({ isActive }) =>
                  `flex items-center gap-3 px-3 py-2 rounded-lg transition-colors
                   ${isActive
                     ? 'bg-bg-tertiary text-text-primary'
                     : 'text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/50'
                   }`
                }
              >
                <Icon size={20} />
                {!collapsed && <span>{label}</span>}
              </NavLink>
            </li>
          ))}
        </ul>
      </nav>

      {/* Bottom section */}
      <div className="border-t border-border p-2">
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            `flex items-center gap-3 px-3 py-2 rounded-lg transition-colors
             ${isActive
               ? 'bg-bg-tertiary text-text-primary'
               : 'text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/50'
             }`
          }
        >
          <Settings size={20} />
          {!collapsed && <span>Settings</span>}
        </NavLink>

        <button
          onClick={handleLogout}
          className="w-full flex items-center gap-3 px-3 py-2 rounded-lg
                     text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/50
                     transition-colors"
        >
          <LogOut size={20} />
          {!collapsed && <span>Logout</span>}
        </button>
      </div>
    </aside>
  );
}
