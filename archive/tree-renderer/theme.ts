import type { TreeTheme } from './types'

const DEFAULT_THEME: TreeTheme = {
  colorPending: '#555555',
  colorRunning: '#eab308',
  colorCompleted: '#22c55e',
  colorFailed: '#ef4444',
  colorWaiting: '#eab308',
  colorSkipped: '#555555',
  colorNodeBg: '#111111',
  colorNodeBorder: '#2a2a2a',
  colorEdge: '#555555',
  colorEdgeActive: '#888888',
  colorLabel: '#e0e0e0',
  colorLabelSecondary: '#888888',
  fontFamily: "'VT323', monospace",
  fontSize: 14,
  nodeRadius: 2,
  glowEnabled: true,
}

const STATUS_COLOR_MAP: Record<string, keyof TreeTheme> = {
  pending: 'colorPending',
  running: 'colorRunning',
  completed: 'colorCompleted',
  failed: 'colorFailed',
  waiting: 'colorWaiting',
  skipped: 'colorSkipped',
}

const getStatusColor = (theme: TreeTheme, status: string): string => {
  const key = STATUS_COLOR_MAP[status]
  if (key !== undefined) return theme[key] as string
  return theme.colorPending
}

const themeToCSS = (theme: TreeTheme): Record<string, string> => ({
  '--tree-color-pending': theme.colorPending,
  '--tree-color-running': theme.colorRunning,
  '--tree-color-completed': theme.colorCompleted,
  '--tree-color-failed': theme.colorFailed,
  '--tree-color-waiting': theme.colorWaiting,
  '--tree-color-skipped': theme.colorSkipped,
  '--tree-node-bg': theme.colorNodeBg,
  '--tree-node-border': theme.colorNodeBorder,
  '--tree-edge-color': theme.colorEdge,
  '--tree-edge-active': theme.colorEdgeActive,
  '--tree-label-color': theme.colorLabel,
  '--tree-label-secondary': theme.colorLabelSecondary,
  '--tree-font-family': theme.fontFamily,
  '--tree-font-size': `${theme.fontSize}px`,
  '--tree-node-radius': `${theme.nodeRadius}px`,
})

export { DEFAULT_THEME, getStatusColor, themeToCSS }
