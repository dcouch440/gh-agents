export const CANVAS = {
  NODE_WIDTH: 260,
  HANDLE_SIZE: 12,
  HANDLE_SIZE_SMALL: 8,
  HANDLE_BORDER_WIDTH: 2,
  GRID_SIZE: 24,
  FIT_VIEW_PADDING: 0.15,
  GROUP_HOVER_DELAY_MS: 300,
} as const

export const STEP_TYPE_COLORS: Record<string, string> = {
  single: '#3b82f6',
  for_each: '#2dd4bf',
  room: '#a78bfa',
  context: '#10b981',
  input: '#f59e0b',
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

/** Shared styling constants for context menus and picker panels. */
export const SECTION_LABEL_SX = {
  px: 1.5,
  py: 0.75,
  fontSize: 10,
  textTransform: 'uppercase',
  color: 'text.disabled',
  letterSpacing: '0.05em',
  fontWeight: 600,
} as const

export const COLOR_DOT_SX = {
  width: 8,
  height: 8,
  borderRadius: '50%',
  flexShrink: 0,
} as const

export const PIPE = {
  // Layer widths (px)
  GLOW_WIDTH: 16,
  BODY_WIDTH: 6,
  CORE_WIDTH: 2,
  PARTICLE_WIDTH: 2,
  BODY_WIDTH_DIM: 4,
  CORE_WIDTH_DIM: 1.5,

  // Opacities — protocol edges
  GLOW_OPACITY: 0.15,
  BODY_OPACITY: 0.35,
  CORE_OPACITY: 0.7,
  PARTICLE_OPACITY: 0.8,

  // Opacities — non-protocol edges
  BODY_OPACITY_DIM: 0.15,
  CORE_OPACITY_DIM: 0.3,
  PARTICLE_OPACITY_DIM: 0.4,

  // Opacities — selected state
  GLOW_OPACITY_SELECTED: 0.3,
  BODY_OPACITY_SELECTED: 0.5,
  CORE_OPACITY_SELECTED: 0.9,
  PARTICLE_OPACITY_SELECTED: 1.0,

  // Particle animation
  PARTICLE_DASH: '4 8',
  PARTICLE_DASH_OFFSET: 12,
  FLOW_DURATION: '1.2s',

  // Inner core brightness boost (0..1 — how much to mix toward white)
  CORE_BRIGHTEN: 0.4,

  // Interaction hit area
  INTERACTION_WIDTH: 20,
} as const
