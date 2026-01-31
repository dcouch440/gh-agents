import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '../store';

export function useKeyboardShortcuts() {
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);
  const navigate = useNavigate();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;

      if (mod && e.key === 'k') {
        e.preventDefault();
        navigate('/chat');
      }

      if (mod && e.key === '/') {
        e.preventDefault();
        toggleSidebar();
      }

      if (e.key === 'Escape') {
        const collapsed = useAppStore.getState().sidebarCollapsed;
        if (!collapsed && window.innerWidth < 1024) {
          toggleSidebar();
        }
      }
    };

    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [toggleSidebar, navigate]);
}
