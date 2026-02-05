import { type ReactNode } from 'react';
import { useLocation } from 'react-router-dom';
import { createElement } from 'react';
import DashboardOutlined from '@mui/icons-material/DashboardOutlined';
import ChatBubbleOutline from '@mui/icons-material/ChatBubbleOutline';
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined';
import AssignmentOutlined from '@mui/icons-material/AssignmentOutlined';
import RateReviewOutlined from '@mui/icons-material/RateReviewOutlined';
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined';
import SettingsOutlined from '@mui/icons-material/SettingsOutlined';
import { ROUTES } from '@/constants';

type NavItem = {
  label: string;
  path: string;
  icon: ReactNode;
};

const NAV_ITEMS: NavItem[] = [
  { label: 'Dashboard', path: ROUTES.DASHBOARD, icon: createElement(DashboardOutlined, { fontSize: 'small' }) },
  { label: 'Chat', path: ROUTES.CHAT, icon: createElement(ChatBubbleOutline, { fontSize: 'small' }) },
  { label: 'Agents', path: ROUTES.AGENTS, icon: createElement(SmartToyOutlined, { fontSize: 'small' }) },
  { label: 'Tasks', path: ROUTES.TASKS, icon: createElement(AssignmentOutlined, { fontSize: 'small' }) },
  { label: 'Review Queue', path: ROUTES.REVIEW_QUEUE, icon: createElement(RateReviewOutlined, { fontSize: 'small' }) },
  { label: 'Documents', path: ROUTES.DOCUMENTS, icon: createElement(DescriptionOutlined, { fontSize: 'small' }) },
  { label: 'Settings', path: ROUTES.SETTINGS, icon: createElement(SettingsOutlined, { fontSize: 'small' }) },
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
