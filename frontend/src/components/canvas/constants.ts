export const CANVAS = {
  NODE_WIDTH: 260,
  HANDLE_SIZE: 12,
  GRID_SIZE: 24,
  FIT_VIEW_PADDING: 0.15,
} as const

export const STEP_TYPE_COLORS: Record<string, string> = {
  single: '#3b82f6',
  for_each: '#2dd4bf',
  room: '#a78bfa',
  cavernous: '#f59e0b',
}

export const DEFAULT_STEP_TYPE_COLOR = '#7d8590'

export const PROTOCOL_TYPE_COLORS: Record<string, string> = {
  decomp: '#3b82f6',
  route: '#a78bfa',
  review: '#f85149',
  transform: '#2dd4bf',
  default: '#7d8590',
}
