import { useLocation, useParams } from 'react-router-dom';
import { Menu, Wifi, WifiOff } from 'lucide-react';
import { useAppStore, useSessionStore } from '../../store';
import { StatusDot } from '../StatusDot';
import { useAgentStatus } from '../../hooks/useAgentStatus';
import { useWebSocketStatus } from '../../hooks/useWebSocketStatus';

const pageTitles: Record<string, string> = {
  '/feed': 'Feed',
  '/tasks': 'Tasks',
  '/agents': 'Agents',
  '/files': 'Files',
  '/stats': 'Stats',
  '/settings': 'Settings',
};

export function Header() {
  const location = useLocation();
  const { sessionId } = useParams<{ sessionId?: string }>();
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);
  const sessions = useSessionStore((s) => s.sessions);
  const { workers, orchestrators } = useAgentStatus();
  const connected = useWebSocketStatus();

  let title = pageTitles[location.pathname] || 'nexor';

  // Show session title when in a chat session
  if (location.pathname.startsWith('/chat/') && sessionId) {
    const session = sessions.find((s) => s.id === sessionId);
    title = session?.title || 'Chat';
  } else if (location.pathname === '/chat') {
    title = 'nexor';
  }

  return (
    <header className="h-14 bg-bg-secondary border-b border-border flex items-center justify-between px-4">
      <div className="flex items-center gap-4">
        <button
          onClick={toggleSidebar}
          className="p-2 rounded-lg text-text-secondary hover:text-text-primary
                     hover:bg-bg-tertiary transition-colors lg:hidden"
        >
          <Menu size={20} />
        </button>
        <h1 className="text-lg font-medium text-text-primary">{title}</h1>
      </div>

      <div className="flex items-center gap-4">
        {/* Agent status */}
        <div className="flex items-center gap-3 text-sm text-text-secondary">
          <span className="flex items-center gap-1">
            <StatusDot status={workers.active > 0 ? 'active' : 'idle'} />
            w[{workers.active}/{workers.total}]
          </span>
          <span className="flex items-center gap-1">
            <StatusDot status={orchestrators.active > 0 ? 'active' : 'idle'} />
            o[{orchestrators.active}/{orchestrators.total}]
          </span>
        </div>

        {/* Connection status */}
        <div className={`flex items-center gap-1 ${connected ? 'text-status-success' : 'text-status-error'}`}>
          {connected ? <Wifi size={16} /> : <WifiOff size={16} />}
        </div>
      </div>
    </header>
  );
}
