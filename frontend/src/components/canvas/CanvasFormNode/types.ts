import type { ReactNode } from 'react'
import type SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import type { HighlightMode } from '../canvasKinds'
import type { ResizeSizeConstraints } from '../ResizableNodeShell'

/**
 * Configuration for a single tab in the horizontal icon tab strip.
 * Content is a fully-rendered ReactNode — the entity owns what goes in each panel.
 */
type CanvasFormTab = {
  id: string
  icon: typeof SettingsOutlined
  tooltip: string
  content: ReactNode
}

type CanvasFormNodeProps = {
  nodeId: string
  header: ReactNode | null
  headerHeight?: number
  tabs: CanvasFormTab[]
  activeTabId: string
  onTabChange: (tabId: string) => void
  selected: boolean
  accentColor?: string
  highlightMode?: HighlightMode
  extraHandles?: ReactNode
  overlay?: ReactNode
  /** When set, overrides the default FORM_NODE size constraints. */
  constraints?: ResizeSizeConstraints
  /** When set, replaces the default left/right handles entirely. */
  handles?: ReactNode
}

export type { CanvasFormTab, CanvasFormNodeProps }
