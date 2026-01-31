import { useState, useEffect } from 'react';
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
  Plus,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { useAuthStore } from '../../store';
import { api, type ModeInfo, type SessionResponse } from '../../api/client';

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
  const [modes, setModes] = useState<ModeInfo[]>([]);
  const [sessions, setSessions] = useState<SessionResponse[]>([]);
  const [sessionsOpen, setSessionsOpen] = useState(false);

  useEffect(() => {
    api.modes.list().then(setModes).catch(() => {});
    api.sessions.list().then(setSessions).catch(() => {});
  }, []);

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const handleCreateSession = async (modeId: string) => {
    try {
      const session = await api.sessions.create(modeId);
      setSessions((prev) => [session, ...prev]);
      navigate(`/chat/${session.id}`);
    } catch {
      // ignore
    }
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
      <nav className="flex-1 py-4 overflow-y-auto">
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

        {/* Sessions section */}
        {!collapsed && (
          <div className="mt-4 px-2">
            <button
              onClick={() => setSessionsOpen(!sessionsOpen)}
              className="flex items-center gap-2 px-3 py-2 w-full text-text-secondary
                         hover:text-text-primary text-xs font-semibold uppercase tracking-wide"
            >
              {sessionsOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              Sessions
            </button>

            {sessionsOpen && (
              <div className="space-y-1">
                {/* New session buttons by mode */}
                {modes.filter((m) => m.id !== 'home').map((mode) => (
                  <button
                    key={mode.id}
                    onClick={() => handleCreateSession(mode.id)}
                    className="flex items-center gap-2 px-3 py-1.5 w-full text-sm rounded-lg
                               text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/50
                               transition-colors"
                  >
                    <Plus size={14} />
                    <span>{mode.name}</span>
                  </button>
                ))}

                {/* Existing sessions */}
                {sessions.length > 0 && (
                  <div className="border-t border-border mt-2 pt-2">
                    {sessions.map((session) => (
                      <NavLink
                        key={session.id}
                        to={`/chat/${session.id}`}
                        className={({ isActive }) =>
                          `block px-3 py-1.5 text-sm rounded-lg truncate transition-colors
                           ${isActive
                             ? 'bg-bg-tertiary text-text-primary'
                             : 'text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/50'
                           }`
                        }
                      >
                        {session.title || session.mode_id}
                      </NavLink>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
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
