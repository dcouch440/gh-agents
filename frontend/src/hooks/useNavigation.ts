import { type ReactNode } from 'react'
import { useLocation } from 'react-router-dom'
import { createElement } from 'react'
import DashboardOutlined from '@mui/icons-material/DashboardOutlined'
import ChatBubbleOutline from '@mui/icons-material/ChatBubbleOutline'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import AssignmentOutlined from '@mui/icons-material/AssignmentOutlined'
import AccountTreeOutlined from '@mui/icons-material/AccountTreeOutlined'
import DescriptionOutlined from '@mui/icons-material/DescriptionOutlined'
import RateReviewOutlined from '@mui/icons-material/RateReviewOutlined'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import { ROUTES } from '@/constants'

type NavGroup = 'nav' | 'utility'

type NavItem = {
  label: string
  path: string
  icon: ReactNode
  group: NavGroup
}

type NavItemWithActive = NavItem & { isActive: boolean }

const NAV_ITEMS: NavItem[] = [
  { label: 'Dashboard', path: ROUTES.DASHBOARD, icon: createElement(DashboardOutlined, { fontSize: 'small' }), group: 'nav' },
  { label: 'Chat', path: ROUTES.CHAT, icon: createElement(ChatBubbleOutline, { fontSize: 'small' }), group: 'nav' },
  { label: 'Agents', path: ROUTES.AGENTS, icon: createElement(SmartToyOutlined, { fontSize: 'small' }), group: 'nav' },
  { label: 'Tasks', path: ROUTES.TASKS, icon: createElement(AssignmentOutlined, { fontSize: 'small' }), group: 'nav' },
  { label: 'Workflows', path: ROUTES.WORKFLOWS, icon: createElement(AccountTreeOutlined, { fontSize: 'small' }), group: 'nav' },
  { label: 'Documents', path: ROUTES.DOCUMENTS, icon: createElement(DescriptionOutlined, { fontSize: 'small' }), group: 'nav' },
  { label: 'Review Queue', path: ROUTES.REVIEW_QUEUE, icon: createElement(RateReviewOutlined, { fontSize: 'small' }), group: 'utility' },
  { label: 'Settings', path: ROUTES.SETTINGS, icon: createElement(SettingsOutlined, { fontSize: 'small' }), group: 'utility' },
]

const isRouteActive = (currentPath: string, routePath: string): boolean => {
  if (routePath === ROUTES.DASHBOARD) {
    return currentPath === routePath
  }
  return currentPath.startsWith(routePath)
}

const useNavigation = () => {
  const location = useLocation()

  const allItems: NavItemWithActive[] = NAV_ITEMS.map((item) => ({
    ...item,
    isActive: isRouteActive(location.pathname, item.path),
  }))

  const navItems = allItems.filter((item) => item.group === 'nav')
  const utilityItems = allItems.filter((item) => item.group === 'utility')

  return { navItems, utilityItems }
}

export { useNavigation }
export type { NavItem, NavItemWithActive, NavGroup }
