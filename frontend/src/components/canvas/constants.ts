export const CANVAS = {
  NODE_WIDTH: 260,
  HANDLE_SIZE: 12,
  GRID_SIZE: 24,
  GRID_DOT_COLOR: 'rgba(255, 255, 255, 0.03)',
  FIT_VIEW_PADDING: 0.15,
} as const

export const STEP_TYPE_COLORS: Record<string, string> = {
  llm: '#3b82f6',
  for_each: '#2dd4bf',
  router: '#a78bfa',
  human: '#f59e0b',
  tool: '#7d8590',
}

export const DEFAULT_STEP_TYPE_COLOR = '#7d8590'
