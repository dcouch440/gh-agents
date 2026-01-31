import { useState, useEffect, useRef } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import {
  Activity,
  ListTodo,
  Users,
  FolderOpen,
  BarChart3,
  Settings,
  LogOut,
  Plus,
  Pencil,
  Trash2,
  Check,
  X,
  Clock,
} from 'lucide-react';
import { useAuthStore, useSessionStore } from '../../store';
import { api } from '../../api/client';
import styles from './Sidebar.module.css';

interface SidebarProps {
  collapsed: boolean;
}

const navItems = [
  { to: '/feed', icon: Activity, label: 'Feed' },
  { to: '/tasks', icon: ListTodo, label: 'Tasks' },
  { to: '/agents', icon: Users, label: 'Agents' },
  { to: '/files', icon: FolderOpen, label: 'Files' },
  { to: '/stats', icon: BarChart3, label: 'Stats' },
];

export function Sidebar({ collapsed }: SidebarProps) {
  const navigate = useNavigate();
  const logout = useAuthStore((state) => state.logout);
  const { sessions, modes, load, addSession, updateSession, removeSession } = useSessionStore();
  const [showNewMenu, setShowNewMenu] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState('');
  const [newSessionId, setNewSessionId] = useState<string | null>(null);
  const editRef = useRef<HTMLInputElement>(null);
  const newMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    if (editingId && editRef.current) {
      editRef.current.focus();
      editRef.current.select();
    }
  }, [editingId]);

  // Close new menu on outside click
  useEffect(() => {
    if (!showNewMenu) return;
    const handler = (e: MouseEvent) => {
      if (newMenuRef.current && !newMenuRef.current.contains(e.target as Node)) {
        setShowNewMenu(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [showNewMenu]);

  // Clear new session animation after it plays
  useEffect(() => {
    if (newSessionId) {
      const timer = setTimeout(() => setNewSessionId(null), 500);
      return () => clearTimeout(timer);
    }
  }, [newSessionId]);

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const handleCreateSession = async (modeId: string) => {
    setShowNewMenu(false);
    try {
      const session = await api.sessions.create(modeId);
      addSession(session);
      setNewSessionId(session.id);
      navigate(`/chat/${session.id}`);
    } catch {
      // ignore
    }
  };

  const handleStartEdit = (session: { id: string; title: string; mode_id: string }) => {
    setEditingId(session.id);
    setEditTitle(session.title || session.mode_id);
  };

  const handleSaveEdit = async () => {
    if (!editingId || !editTitle.trim()) return;
    try {
      const updated = await api.sessions.update(editingId, editTitle.trim());
      updateSession(editingId, updated);
    } catch {
      // ignore
    }
    setEditingId(null);
  };

  const handleCancelEdit = () => {
    setEditingId(null);
  };

  const handleDelete = async (sessionId: string) => {
    try {
      await api.sessions.delete(sessionId);
      removeSession(sessionId);
      navigate('/chat');
    } catch {
      // ignore
    }
  };

  const handleEditKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSaveEdit();
    if (e.key === 'Escape') handleCancelEdit();
  };

  const availableModes = modes.filter((m) => m.id !== 'home');

  return (
    <aside
      className={`${styles.sidebar} ${collapsed ? styles.collapsed : ''}`}
    >
      {/* Logo */}
      <div className={styles.logo}>
        <NavLink to="/chat" className={styles.logoLink}>
          <span className={styles.logoText}>
            {collapsed ? 'n' : 'nexor'}
          </span>
        </NavLink>
      </div>

      {/* Navigation */}
      <nav className={styles.nav}>
        <ul className={styles.navList}>
          {navItems.map(({ to, icon: Icon, label }) => (
            <li key={to}>
              <NavLink
                to={to}
                className={({ isActive }) =>
                  `${styles.navItem} ${isActive ? styles.navItemActive : ''}`
                }
              >
                <Icon size={18} />
                {!collapsed && <span>{label}</span>}
              </NavLink>
            </li>
          ))}
        </ul>

        {/* History section */}
        {!collapsed && (
          <div className={styles.history}>
            <div className={styles.historyHeader}>
              <div className={styles.historyTitle}>
                <Clock size={12} />
                <span>History</span>
              </div>
              <div className={styles.newMenuWrapper} ref={newMenuRef}>
                <button
                  className={styles.newBtn}
                  onClick={() => setShowNewMenu(!showNewMenu)}
                  title="New session"
                >
                  <Plus size={14} />
                </button>
                {showNewMenu && (
                  <div className={styles.newMenu}>
                    {availableModes.map((mode) => (
                      <button
                        key={mode.id}
                        className={styles.newMenuItem}
                        onClick={() => handleCreateSession(mode.id)}
                      >
                        <span className={styles.newMenuName}>{mode.name}</span>
                        <span className={styles.newMenuDesc}>{mode.description}</span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>

            <div className={styles.sessionList}>
              {sessions.length === 0 && (
                <p className={styles.emptyHistory}>No sessions yet</p>
              )}
              {sessions.map((session) => (
                <div
                  key={session.id}
                  className={`${styles.sessionItem} ${newSessionId === session.id ? styles.sessionNew : ''}`}
                >
                  {editingId === session.id ? (
                    <div className={styles.sessionEdit}>
                      <input
                        ref={editRef}
                        value={editTitle}
                        onChange={(e) => setEditTitle(e.target.value)}
                        onKeyDown={handleEditKeyDown}
                        onBlur={handleSaveEdit}
                        className={styles.sessionEditInput}
                      />
                      <button onClick={handleSaveEdit} className={styles.sessionEditSave}>
                        <Check size={12} />
                      </button>
                      <button
                        onMouseDown={(e) => { e.preventDefault(); handleCancelEdit(); }}
                        className={styles.sessionEditCancel}
                      >
                        <X size={12} />
                      </button>
                    </div>
                  ) : (
                    <NavLink
                      to={`/chat/${session.id}`}
                      className={({ isActive }) =>
                        `${styles.sessionLink} ${isActive ? styles.sessionLinkActive : ''}`
                      }
                    >
                      {session.title || session.mode_id}
                    </NavLink>
                  )}
                  {editingId !== session.id && (
                    <div className={styles.sessionActions}>
                      <button
                        onClick={(e) => { e.preventDefault(); handleStartEdit(session); }}
                        className={styles.sessionActionBtn}
                        title="Rename"
                      >
                        <Pencil size={11} />
                      </button>
                      <button
                        onClick={(e) => { e.preventDefault(); handleDelete(session.id); }}
                        className={`${styles.sessionActionBtn} ${styles.sessionDeleteBtn}`}
                        title="Delete"
                      >
                        <Trash2 size={11} />
                      </button>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </nav>

      {/* Bottom section */}
      <div className={styles.bottom}>
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            `${styles.navItem} ${isActive ? styles.navItemActive : ''}`
          }
        >
          <Settings size={18} />
          {!collapsed && <span>Settings</span>}
        </NavLink>

        <button onClick={handleLogout} className={styles.navItem}>
          <LogOut size={18} />
          {!collapsed && <span>Logout</span>}
        </button>
      </div>
    </aside>
  );
}
