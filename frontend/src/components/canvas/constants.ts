export const CANVAS = {
  NODE_WIDTH: 260,
  HANDLE_SIZE: 12,
  HANDLE_SIZE_SMALL: 8,
  HANDLE_BORDER_WIDTH: 2,
  GRID_SIZE: 24,
  FIT_VIEW_PADDING: 0.15,
  EDGE_STROKE_WIDTH: 2.5,
  EDGE_DASH_PATTERN: '6 4',
  EDGE_OPACITY_DEFAULT: 0.4,
  EDGE_OPACITY_PROTOCOL: 0.6,
  EDGE_OPACITY_SELECTED: 0.8,
  EDGE_FLOW_DURATION: '0.6s',
  GROUP_HOVER_DELAY_MS: 300,
} as const

export const STEP_TYPE_COLORS: Record<string, string> = {
  single: '#3b82f6',
  for_each: '#2dd4bf',
  room: '#a78bfa',
  context: '#10b981',
}

export const DEFAULT_STEP_TYPE_COLOR = '#7d8590'

export const GREYSCALE_ACCENT = '#7d8590'

export const PROTOCOL_TYPE_COLORS: Record<string, string> = {
  decomp: '#3b82f6',
  route: '#a78bfa',
  review: '#f85149',
  transform: '#2dd4bf',
  documenter: '#D4793E',
  default: '#7d8590',
}

export const PROTOCOL_LABELS: Record<string, string> = {
  documenter: 'Documenter',
}
