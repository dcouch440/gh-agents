import { useCallback, useState } from 'react';
import { LS_SIDEBAR_COLLAPSED } from '@/constants';

const getInitialCollapsed = (): boolean => {
  const stored = localStorage.getItem(LS_SIDEBAR_COLLAPSED);
  return stored === 'true';
};

const useSidebar = () => {
  const [collapsed, setCollapsedState] = useState(getInitialCollapsed);

  const setCollapsed = useCallback((value: boolean) => {
    setCollapsedState(value);
    localStorage.setItem(LS_SIDEBAR_COLLAPSED, String(value));
  }, []);

  const toggle = useCallback(() => {
    setCollapsed(!collapsed);
  }, [collapsed, setCollapsed]);

  return { collapsed, toggle, setCollapsed };
};

export { useSidebar };
