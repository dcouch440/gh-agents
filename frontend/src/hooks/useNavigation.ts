import { useLocation } from 'react-router-dom';
import { ROUTES } from '@/constants';

type NavItem = {
  label: string;
  path: string;
  icon?: React.ReactNode;
};

const NAV_ITEMS: NavItem[] = [
  { label: 'Dashboard', path: ROUTES.DASHBOARD },
  { label: 'Chat', path: ROUTES.CHAT },
  { label: 'Agents', path: ROUTES.AGENTS },
  { label: 'Pipelines', path: ROUTES.PIPELINES },
  { label: 'Tasks', path: ROUTES.TASKS },
  { label: 'Documents', path: ROUTES.DOCUMENTS },
  { label: 'Settings', path: ROUTES.SETTINGS },
];

// Base-level helper function
const isRouteActive = (currentPath: string, routePath: string): boolean => {
  if (routePath === ROUTES.DASHBOARD) {
    return currentPath === routePath;
  }
  return currentPath.startsWith(routePath);
};

const useNavigation = () => {
  const location = useLocation();

  const navItems = NAV_ITEMS.map((item) => ({
    ...item,
    isActive: isRouteActive(location.pathname, item.path),
  }));

  return { navItems };
};

export { useNavigation };
export type { NavItem };
