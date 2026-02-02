import { Outlet, useLocation } from 'react-router-dom';
import { Sidebar } from '../Sidebar';
import { Header } from '../Header';
import { useAppStore } from '../../store';
import { useKeyboardShortcuts } from '../../hooks/useKeyboardShortcuts';
import { useWebSocket } from '../../hooks/useWebSocket';

export function Layout() {
  useKeyboardShortcuts();
  useWebSocket();
  const sidebarCollapsed = useAppStore((state) => state.sidebarCollapsed);
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);
  const location = useLocation();

  const isChat = location.pathname === '/chat' || location.pathname === '/' || location.pathname.startsWith('/chat/');
  const fullWidthPaths = ['/agents', '/tasks', '/files', '/stats', '/settings'];
  const isFullWidth = isChat || fullWidthPaths.some((p) => location.pathname.startsWith(p));
  const contentMaxWidth = isFullWidth ? '' : 'max-w-4xl';

  return (
    <div className="h-screen bg-bg-primary flex overflow-hidden">
      {/* Mobile overlay */}
      {!sidebarCollapsed && (
        <div
          className="fixed inset-0 bg-black/50 z-30 lg:hidden"
          onClick={toggleSidebar}
        />
      )}

      <Sidebar collapsed={sidebarCollapsed} />

      {/* Main content area */}
      <div className="flex-1 flex flex-col min-w-0">
        <Header />
        {isChat ? (
          <main className="flex-1 overflow-hidden">
            <Outlet />
          </main>
        ) : (
          <main className="flex-1 overflow-auto p-6">
            <div className={`${contentMaxWidth} mx-auto`}>
              <Outlet />
            </div>
          </main>
        )}
      </div>
    </div>
  );
}
