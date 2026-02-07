import { useStore } from '@/stores/lib';
import { uiStore } from '@/stores/uiStore';

const useSidebar = () => {
  const collapsed = useStore(uiStore.store, uiStore.selectSidebarCollapsed);

  return {
    collapsed,
    toggle: uiStore.toggleSidebar,
    setCollapsed: uiStore.setSidebarCollapsed,
  };
};

export { useSidebar };
