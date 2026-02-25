import type { PaletteOptions, Shadows } from '@mui/material/styles'
import type { CustomTokens } from './customTokens'

// ---------------------------------------------------------------------------
// Theme identity
// ---------------------------------------------------------------------------

type ThemeId = 'linen' | 'paper' | 'obsidian' | 'midnight' | 'slate'

const THEME_IDS: readonly ThemeId[] = ['linen', 'paper', 'obsidian', 'midnight', 'slate'] as const

const DEFAULT_THEME_ID: ThemeId = 'linen'

// ---------------------------------------------------------------------------
// Node palette — per-variant accent colors within a theme
// ---------------------------------------------------------------------------

type NodePalette = {
  workforce: string
  manager: string
  room: string
  blank: string
  agent: string
  context: string
  input: string
  step: string
  sub_workflow: string
}

// ---------------------------------------------------------------------------
// Theme definition
// ---------------------------------------------------------------------------

type ThemeDefinition = {
  id: ThemeId
  label: string
  muiMode: 'light' | 'dark'
  palette: PaletteOptions
  custom: CustomTokens
  shadows: Shadows
  nodePalette: NodePalette
}

// ═══════════════════════════════════════════════════════════════════════════
// Linen — Warm light theme (current light)
// ═══════════════════════════════════════════════════════════════════════════

const linenPalette: PaletteOptions = {
  mode: 'light',
  primary: { main: '#FF964F', light: '#FFB480', dark: '#D47830', contrastText: '#2D1B0E' },
  secondary: { main: '#8B6548', light: '#A88066', dark: '#725438' },
  background: { default: '#F9F6F1', paper: '#FEFCFA' },
  text: { primary: '#2D1B0E', secondary: '#6B5742', disabled: '#A39283' },
  success: { main: '#4E8A5A', light: '#6BA878', dark: '#3B7046' },
  warning: { main: '#B87312', light: '#D49030', dark: '#955C0A' },
  error: { main: '#BF3326', light: '#D94A3D', dark: '#952820' },
  info: { main: '#FF964F', light: '#FFB480', dark: '#D47830' },
  divider: 'rgba(45, 27, 14, 0.10)',
}

const linenCustom: CustomTokens = {
  chromeBg: '#D47830',
  cavityBg: '#F5F3F0',
  bgHeader: '#FAF9F7',
  bgEditor: '#FCFBFA',
  bgPanel: '#F8F7F5',
  borderHover: 'rgba(45, 27, 14, 0.15)',
  textFaint: '#CFC7BB',
  hoverOverlay: 'rgba(45, 27, 14, 0.04)',
  separatorSubtle: 'rgba(45, 27, 14, 0.06)',
  activeTint: 'rgba(255, 150, 79, 0.08)',
  activeTintStrong: 'rgba(255, 150, 79, 0.14)',
  activeGradient: 'linear-gradient(90deg, #FF964F, #4E8A5A)',
  activeGradientVertical: 'linear-gradient(180deg, #FF964F, #4E8A5A)',
  activeGlow: 'drop-shadow(0 0 6px rgba(255, 150, 79, 0.35))',
  gridDotColor: 'rgba(180, 170, 158, 0.15)',
  gridLineColor: 'rgba(180, 170, 158, 0.06)',
  canvasVignette: 'rgba(60, 50, 40, 0.06)',
  canvasGlow: 'rgba(250, 248, 245, 0.25)',
  minimapBg: 'rgba(240, 235, 227, 0.9)',
  minimapMask: 'rgba(249, 246, 241, 0.7)',
  floatingPanelBg: 'rgba(254, 252, 250, 0.96)',
  floatingPanelBorder: 'rgba(45, 27, 14, 0.12)',
  chromeText: 'rgba(254, 252, 250, 0.70)',
  chromeTextActive: '#FEFCFA',
  chromeTextHover: 'rgba(254, 252, 250, 0.90)',
  chromeActiveGlow: 'drop-shadow(0 0 4px rgba(255, 150, 79, 0.4))',
  chromeActiveBar: '#FEFCFA',
  screenBg: '#f0ebe3',
  screenBorder: '#d6cfc4',
  screenFrost: 'rgba(240, 235, 227, 0.8)',
  canvasBg: '#eae5da',
  accent: '#5a8a6e',
  accentBg: 'rgba(90, 138, 110, 0.10)',
  accentRing: 'rgba(90, 138, 110, 0.18)',
  userBarBg: 'rgba(90, 138, 110, 0.09)',
  connectorColor: '#cdc6ba',
  strokeColor: '#1e1e1e',
  surfaceBg: '#FEFCFA',
  elevatedBg: '#FEFCFA',
  appBarBg: '#F4F0EA',
}

const linenShadows: Shadows = [
  'none',
  '0 1px 2px rgba(45, 27, 14, 0.04)',
  '0 1px 3px rgba(45, 27, 14, 0.05)',
  '0 2px 4px rgba(45, 27, 14, 0.05)',
  '0 2px 6px rgba(45, 27, 14, 0.06)',
  '0 3px 8px rgba(45, 27, 14, 0.06)',
  '0 4px 10px rgba(45, 27, 14, 0.07)',
  '0 4px 12px rgba(45, 27, 14, 0.07)',
  '0 6px 14px rgba(45, 27, 14, 0.08)',
  '0 6px 16px rgba(45, 27, 14, 0.08)',
  '0 8px 18px rgba(45, 27, 14, 0.08)',
  '0 8px 20px rgba(45, 27, 14, 0.09)',
  '0 10px 22px rgba(45, 27, 14, 0.09)',
  '0 10px 24px rgba(45, 27, 14, 0.09)',
  '0 12px 26px rgba(45, 27, 14, 0.10)',
  '0 12px 28px rgba(45, 27, 14, 0.10)',
  '0 14px 30px rgba(45, 27, 14, 0.11)',
  '0 14px 32px rgba(45, 27, 14, 0.11)',
  '0 16px 34px rgba(45, 27, 14, 0.11)',
  '0 16px 36px rgba(45, 27, 14, 0.11)',
  '0 18px 38px rgba(45, 27, 14, 0.12)',
  '0 18px 40px rgba(45, 27, 14, 0.12)',
  '0 20px 42px rgba(45, 27, 14, 0.13)',
  '0 20px 44px rgba(45, 27, 14, 0.13)',
  '0 22px 46px rgba(45, 27, 14, 0.13)',
]

const linenNodePalette: NodePalette = {
  workforce: '#4E7B9B',
  manager: '#7C5CAF',
  room: '#9B7BB8',
  blank: '#8B8178',
  agent: '#3D9E9E',
  context: '#5A8A6E',
  input: '#C4882A',
  step: '#8B8178',
  sub_workflow: '#5A8A6E',
}

// ═══════════════════════════════════════════════════════════════════════════
// Midnight — Deep dark theme (current dark)
// ═══════════════════════════════════════════════════════════════════════════

const midnightPalette: PaletteOptions = {
  mode: 'dark',
  primary: { main: '#3b82f6', light: '#60a5fa', dark: '#2563eb', contrastText: '#ffffff' },
  secondary: { main: '#2dd4bf', light: '#5eead4', dark: '#14b8a6' },
  background: { default: '#0d1117', paper: '#21262d' },
  text: { primary: '#f0f6fc', secondary: '#8b949e', disabled: '#6e7681' },
  success: { main: '#3fb950', light: '#56d364', dark: '#2ea043' },
  warning: { main: '#d29922', light: '#e3b341', dark: '#bb8009' },
  error: { main: '#f85149', light: '#ff7b72', dark: '#da3633' },
  info: { main: '#3b82f6', light: '#60a5fa', dark: '#2563eb' },
  divider: 'rgba(240, 246, 252, 0.10)',
}

const midnightCustom: CustomTokens = {
  // Surface hierarchy: 0d1117 → 161b22 → 1c2128 → 21262d → 2d333b
  chromeBg: '#161b22',
  cavityBg: '#0d1117',
  bgHeader: '#21262d',
  bgEditor: '#161b22',
  bgPanel: '#1c2128',
  borderHover: 'rgba(240, 246, 252, 0.15)',
  textFaint: '#30363d',
  hoverOverlay: 'rgba(255, 255, 255, 0.04)',
  separatorSubtle: 'rgba(240, 246, 252, 0.04)',
  activeTint: 'rgba(59, 130, 246, 0.08)',
  activeTintStrong: 'rgba(59, 130, 246, 0.12)',
  activeGradient: 'linear-gradient(90deg, #3b82f6, #2dd4bf)',
  activeGradientVertical: 'linear-gradient(180deg, #3b82f6, #2dd4bf)',
  activeGlow: 'drop-shadow(0 0 4px rgba(59, 130, 246, 0.4))',
  gridDotColor: 'rgba(240, 246, 252, 0.06)',
  gridLineColor: 'rgba(240, 246, 252, 0.03)',
  canvasVignette: 'rgba(0, 0, 0, 0.15)',
  canvasGlow: 'transparent',
  minimapBg: 'rgba(13, 17, 23, 0.9)',
  minimapMask: 'rgba(13, 17, 23, 0.7)',
  floatingPanelBg: 'rgba(22, 27, 34, 0.95)',
  floatingPanelBorder: 'rgba(240, 246, 252, 0.1)',
  chromeText: '#8b949e',
  chromeTextActive: '#3b82f6',
  chromeTextHover: '#f0f6fc',
  chromeActiveGlow: 'none',
  chromeActiveBar: '#3b82f6',
  screenBg: '#161b22',
  screenBorder: '#30363d',
  screenFrost: 'rgba(13, 17, 23, 0.8)',
  canvasBg: '#0d1117',
  accent: '#3b82f6',
  accentBg: 'rgba(59, 130, 246, 0.10)',
  accentRing: 'rgba(59, 130, 246, 0.35)',
  userBarBg: 'rgba(59, 130, 246, 0.08)',
  connectorColor: '#30363d',
  strokeColor: '#c9d1d9',
  surfaceBg: '#21262d',
  elevatedBg: '#2d333b',
  appBarBg: '#161b22',
}

const midnightShadows: Shadows = [
  'none',
  '0 1px 2px rgba(0, 0, 0, 0.30)',
  '0 1px 3px rgba(0, 0, 0, 0.32)',
  '0 2px 4px rgba(0, 0, 0, 0.32)',
  '0 2px 6px rgba(0, 0, 0, 0.34)',
  '0 3px 8px rgba(0, 0, 0, 0.34)',
  '0 4px 10px rgba(0, 0, 0, 0.36)',
  '0 4px 12px rgba(0, 0, 0, 0.36)',
  '0 6px 14px rgba(0, 0, 0, 0.38)',
  '0 6px 16px rgba(0, 0, 0, 0.38)',
  '0 8px 18px rgba(0, 0, 0, 0.38)',
  '0 8px 20px rgba(0, 0, 0, 0.40)',
  '0 10px 22px rgba(0, 0, 0, 0.40)',
  '0 10px 24px rgba(0, 0, 0, 0.40)',
  '0 12px 26px rgba(0, 0, 0, 0.42)',
  '0 12px 28px rgba(0, 0, 0, 0.42)',
  '0 14px 30px rgba(0, 0, 0, 0.44)',
  '0 14px 32px rgba(0, 0, 0, 0.44)',
  '0 16px 34px rgba(0, 0, 0, 0.44)',
  '0 16px 36px rgba(0, 0, 0, 0.44)',
  '0 18px 38px rgba(0, 0, 0, 0.46)',
  '0 18px 40px rgba(0, 0, 0, 0.46)',
  '0 20px 42px rgba(0, 0, 0, 0.48)',
  '0 20px 44px rgba(0, 0, 0, 0.48)',
  '0 22px 46px rgba(0, 0, 0, 0.50)',
]

const midnightNodePalette: NodePalette = {
  workforce: '#3b82f6',
  manager: '#8b5cf6',
  room: '#a78bfa',
  blank: '#7d8590',
  agent: '#06b6d4',
  context: '#10b981',
  input: '#f59e0b',
  step: '#7d8590',
  sub_workflow: '#10b981',
}

// ═══════════════════════════════════════════════════════════════════════════
// Slate — Cool professional mid-tone
// ═══════════════════════════════════════════════════════════════════════════

const slatePalette: PaletteOptions = {
  mode: 'dark',
  primary: { main: '#818cf8', light: '#a5b4fc', dark: '#6366f1', contrastText: '#ffffff' },
  secondary: { main: '#67e8f9', light: '#a5f3fc', dark: '#22d3ee' },
  background: { default: '#1a1f2e', paper: '#282f40' },
  text: { primary: '#e2e8f0', secondary: '#94a3b8', disabled: '#7c8ca0' },
  success: { main: '#34d399', light: '#6ee7b7', dark: '#10b981' },
  warning: { main: '#fbbf24', light: '#fcd34d', dark: '#f59e0b' },
  error: { main: '#fb7185', light: '#fda4af', dark: '#f43f5e' },
  info: { main: '#818cf8', light: '#a5b4fc', dark: '#6366f1' },
  divider: 'rgba(226, 232, 240, 0.10)',
}

const slateCustom: CustomTokens = {
  // Surface hierarchy: 171c2a → 1a1f2e → 1e2433 → 232a3b → 282f40 → 323a4e
  chromeBg: '#1e2433',
  cavityBg: '#1a1f2e',
  bgHeader: '#282f40',
  bgEditor: '#1e2433',
  bgPanel: '#232a3b',
  borderHover: 'rgba(226, 232, 240, 0.14)',
  textFaint: '#475569',
  hoverOverlay: 'rgba(255, 255, 255, 0.05)',
  separatorSubtle: 'rgba(226, 232, 240, 0.05)',
  activeTint: 'rgba(129, 140, 248, 0.08)',
  activeTintStrong: 'rgba(129, 140, 248, 0.14)',
  activeGradient: 'linear-gradient(90deg, #818cf8, #22d3ee)',
  activeGradientVertical: 'linear-gradient(180deg, #818cf8, #22d3ee)',
  activeGlow: 'drop-shadow(0 0 4px rgba(129, 140, 248, 0.4))',
  gridDotColor: 'rgba(226, 232, 240, 0.06)',
  gridLineColor: 'rgba(226, 232, 240, 0.03)',
  canvasVignette: 'rgba(0, 0, 0, 0.10)',
  canvasGlow: 'transparent',
  minimapBg: 'rgba(23, 28, 42, 0.9)',
  minimapMask: 'rgba(26, 31, 46, 0.7)',
  floatingPanelBg: 'rgba(30, 36, 51, 0.95)',
  floatingPanelBorder: 'rgba(226, 232, 240, 0.10)',
  chromeText: '#94a3b8',
  chromeTextActive: '#818cf8',
  chromeTextHover: '#e2e8f0',
  chromeActiveGlow: 'none',
  chromeActiveBar: '#818cf8',
  screenBg: '#1e2433',
  screenBorder: '#364152',
  screenFrost: 'rgba(26, 31, 46, 0.8)',
  canvasBg: '#171c2a',
  accent: '#818cf8',
  accentBg: 'rgba(129, 140, 248, 0.10)',
  accentRing: 'rgba(129, 140, 248, 0.30)',
  userBarBg: 'rgba(129, 140, 248, 0.08)',
  connectorColor: '#475569',
  strokeColor: '#e2e8f0',
  surfaceBg: '#282f40',
  elevatedBg: '#323a4e',
  appBarBg: '#1e2433',
}

const slateShadows: Shadows = [
  'none',
  '0 1px 2px rgba(0, 0, 0, 0.22)',
  '0 1px 3px rgba(0, 0, 0, 0.24)',
  '0 2px 4px rgba(0, 0, 0, 0.24)',
  '0 2px 6px rgba(0, 0, 0, 0.26)',
  '0 3px 8px rgba(0, 0, 0, 0.26)',
  '0 4px 10px rgba(0, 0, 0, 0.28)',
  '0 4px 12px rgba(0, 0, 0, 0.28)',
  '0 6px 14px rgba(0, 0, 0, 0.30)',
  '0 6px 16px rgba(0, 0, 0, 0.30)',
  '0 8px 18px rgba(0, 0, 0, 0.30)',
  '0 8px 20px rgba(0, 0, 0, 0.32)',
  '0 10px 22px rgba(0, 0, 0, 0.32)',
  '0 10px 24px rgba(0, 0, 0, 0.32)',
  '0 12px 26px rgba(0, 0, 0, 0.34)',
  '0 12px 28px rgba(0, 0, 0, 0.34)',
  '0 14px 30px rgba(0, 0, 0, 0.36)',
  '0 14px 32px rgba(0, 0, 0, 0.36)',
  '0 16px 34px rgba(0, 0, 0, 0.36)',
  '0 16px 36px rgba(0, 0, 0, 0.36)',
  '0 18px 38px rgba(0, 0, 0, 0.38)',
  '0 18px 40px rgba(0, 0, 0, 0.38)',
  '0 20px 42px rgba(0, 0, 0, 0.40)',
  '0 20px 44px rgba(0, 0, 0, 0.40)',
  '0 22px 46px rgba(0, 0, 0, 0.42)',
]

const slateNodePalette: NodePalette = {
  workforce: '#818cf8',
  manager: '#a78bfa',
  room: '#c084fc',
  blank: '#94a3b8',
  agent: '#22d3ee',
  context: '#34d399',
  input: '#fbbf24',
  step: '#94a3b8',
  sub_workflow: '#34d399',
}

// ═══════════════════════════════════════════════════════════════════════════
// Obsidian — Pitch black + white strokes
// ═══════════════════════════════════════════════════════════════════════════

const obsidianPalette: PaletteOptions = {
  mode: 'dark',
  primary: { main: '#888888', light: '#aaaaaa', dark: '#666666', contrastText: '#000000' },
  secondary: { main: '#999999', light: '#bbbbbb', dark: '#777777' },
  background: { default: '#000000', paper: '#1a1a1a' },
  text: { primary: '#ffffff', secondary: '#aaaaaa', disabled: '#666666' },
  success: { main: '#4ade80', light: '#86efac', dark: '#22c55e' },
  warning: { main: '#fbbf24', light: '#fcd34d', dark: '#f59e0b' },
  error: { main: '#f87171', light: '#fca5a5', dark: '#ef4444' },
  info: { main: '#888888', light: '#aaaaaa', dark: '#666666' },
  divider: 'rgba(255, 255, 255, 0.12)',
}

const obsidianCustom: CustomTokens = {
  chromeBg: '#0a0a0a',
  cavityBg: '#000000',
  bgHeader: '#1a1a1a',
  bgEditor: '#0a0a0a',
  bgPanel: '#111111',
  borderHover: 'rgba(255, 255, 255, 0.20)',
  textFaint: '#333333',
  hoverOverlay: 'rgba(255, 255, 255, 0.06)',
  separatorSubtle: 'rgba(255, 255, 255, 0.06)',
  activeTint: 'rgba(136, 136, 136, 0.10)',
  activeTintStrong: 'rgba(136, 136, 136, 0.16)',
  activeGradient: 'linear-gradient(90deg, #888888, #aaaaaa)',
  activeGradientVertical: 'linear-gradient(180deg, #888888, #aaaaaa)',
  activeGlow: 'drop-shadow(0 0 4px rgba(136, 136, 136, 0.3))',
  gridDotColor: 'rgba(255, 255, 255, 0.06)',
  gridLineColor: 'rgba(255, 255, 255, 0.03)',
  canvasVignette: 'rgba(0, 0, 0, 0.0)',
  canvasGlow: 'transparent',
  minimapBg: 'rgba(0, 0, 0, 0.9)',
  minimapMask: 'rgba(0, 0, 0, 0.7)',
  floatingPanelBg: 'rgba(10, 10, 10, 0.95)',
  floatingPanelBorder: 'rgba(255, 255, 255, 0.12)',
  chromeText: '#777777',
  chromeTextActive: '#ffffff',
  chromeTextHover: '#cccccc',
  chromeActiveGlow: 'none',
  chromeActiveBar: '#ffffff',
  screenBg: '#0a0a0a',
  screenBorder: '#333333',
  screenFrost: 'rgba(0, 0, 0, 0.8)',
  canvasBg: '#000000',
  accent: '#888888',
  accentBg: 'rgba(136, 136, 136, 0.10)',
  accentRing: 'rgba(136, 136, 136, 0.30)',
  userBarBg: 'rgba(136, 136, 136, 0.08)',
  connectorColor: '#444444',
  strokeColor: '#ffffff',
  surfaceBg: '#000000',
  elevatedBg: '#1a1a1a',
  appBarBg: '#0a0a0a',
}

const obsidianShadows: Shadows = [
  'none',
  '0 1px 2px rgba(255, 255, 255, 0.04)',
  '0 1px 3px rgba(255, 255, 255, 0.05)',
  '0 2px 4px rgba(255, 255, 255, 0.05)',
  '0 2px 6px rgba(255, 255, 255, 0.06)',
  '0 3px 8px rgba(255, 255, 255, 0.06)',
  '0 4px 10px rgba(255, 255, 255, 0.07)',
  '0 4px 12px rgba(255, 255, 255, 0.07)',
  '0 6px 14px rgba(255, 255, 255, 0.08)',
  '0 6px 16px rgba(255, 255, 255, 0.08)',
  '0 8px 18px rgba(255, 255, 255, 0.08)',
  '0 8px 20px rgba(255, 255, 255, 0.09)',
  '0 10px 22px rgba(255, 255, 255, 0.09)',
  '0 10px 24px rgba(255, 255, 255, 0.09)',
  '0 12px 26px rgba(255, 255, 255, 0.10)',
  '0 12px 28px rgba(255, 255, 255, 0.10)',
  '0 14px 30px rgba(255, 255, 255, 0.11)',
  '0 14px 32px rgba(255, 255, 255, 0.11)',
  '0 16px 34px rgba(255, 255, 255, 0.11)',
  '0 16px 36px rgba(255, 255, 255, 0.11)',
  '0 18px 38px rgba(255, 255, 255, 0.12)',
  '0 18px 40px rgba(255, 255, 255, 0.12)',
  '0 20px 42px rgba(255, 255, 255, 0.13)',
  '0 20px 44px rgba(255, 255, 255, 0.13)',
  '0 22px 46px rgba(255, 255, 255, 0.13)',
]

const obsidianNodePalette: NodePalette = {
  workforce: '#888888',
  manager: '#aaaaaa',
  room: '#999999',
  blank: '#666666',
  agent: '#bbbbbb',
  context: '#999999',
  input: '#cccccc',
  step: '#666666',
  sub_workflow: '#999999',
}

// ═══════════════════════════════════════════════════════════════════════════
// Paper — Pure white + black strokes
// ═══════════════════════════════════════════════════════════════════════════

const paperPalette: PaletteOptions = {
  mode: 'light',
  primary: { main: '#666666', light: '#888888', dark: '#444444', contrastText: '#ffffff' },
  secondary: { main: '#777777', light: '#999999', dark: '#555555' },
  background: { default: '#ffffff', paper: '#f5f5f5' },
  text: { primary: '#000000', secondary: '#555555', disabled: '#999999' },
  success: { main: '#16a34a', light: '#22c55e', dark: '#15803d' },
  warning: { main: '#ca8a04', light: '#eab308', dark: '#a16207' },
  error: { main: '#dc2626', light: '#ef4444', dark: '#b91c1c' },
  info: { main: '#666666', light: '#888888', dark: '#444444' },
  divider: 'rgba(0, 0, 0, 0.10)',
}

const paperCustom: CustomTokens = {
  chromeBg: '#f0f0f0',
  cavityBg: '#ffffff',
  bgHeader: '#f5f5f5',
  bgEditor: '#fafafa',
  bgPanel: '#f7f7f7',
  borderHover: 'rgba(0, 0, 0, 0.15)',
  textFaint: '#cccccc',
  hoverOverlay: 'rgba(0, 0, 0, 0.04)',
  separatorSubtle: 'rgba(0, 0, 0, 0.06)',
  activeTint: 'rgba(102, 102, 102, 0.08)',
  activeTintStrong: 'rgba(102, 102, 102, 0.14)',
  activeGradient: 'linear-gradient(90deg, #666666, #888888)',
  activeGradientVertical: 'linear-gradient(180deg, #666666, #888888)',
  activeGlow: 'drop-shadow(0 0 4px rgba(102, 102, 102, 0.3))',
  gridDotColor: 'rgba(0, 0, 0, 0.08)',
  gridLineColor: 'rgba(0, 0, 0, 0.04)',
  canvasVignette: 'rgba(0, 0, 0, 0.0)',
  canvasGlow: 'transparent',
  minimapBg: 'rgba(255, 255, 255, 0.9)',
  minimapMask: 'rgba(255, 255, 255, 0.7)',
  floatingPanelBg: 'rgba(250, 250, 250, 0.96)',
  floatingPanelBorder: 'rgba(0, 0, 0, 0.10)',
  chromeText: '#888888',
  chromeTextActive: '#000000',
  chromeTextHover: '#333333',
  chromeActiveGlow: 'none',
  chromeActiveBar: '#000000',
  screenBg: '#f0f0f0',
  screenBorder: '#cccccc',
  screenFrost: 'rgba(255, 255, 255, 0.8)',
  canvasBg: '#ffffff',
  accent: '#666666',
  accentBg: 'rgba(102, 102, 102, 0.08)',
  accentRing: 'rgba(102, 102, 102, 0.20)',
  userBarBg: 'rgba(102, 102, 102, 0.06)',
  connectorColor: '#cccccc',
  strokeColor: '#000000',
  surfaceBg: '#ffffff',
  elevatedBg: '#f5f5f5',
  appBarBg: '#f0f0f0',
}

const paperShadows: Shadows = [
  'none',
  '0 1px 2px rgba(0, 0, 0, 0.06)',
  '0 1px 3px rgba(0, 0, 0, 0.07)',
  '0 2px 4px rgba(0, 0, 0, 0.07)',
  '0 2px 6px rgba(0, 0, 0, 0.08)',
  '0 3px 8px rgba(0, 0, 0, 0.08)',
  '0 4px 10px rgba(0, 0, 0, 0.09)',
  '0 4px 12px rgba(0, 0, 0, 0.09)',
  '0 6px 14px rgba(0, 0, 0, 0.10)',
  '0 6px 16px rgba(0, 0, 0, 0.10)',
  '0 8px 18px rgba(0, 0, 0, 0.10)',
  '0 8px 20px rgba(0, 0, 0, 0.11)',
  '0 10px 22px rgba(0, 0, 0, 0.11)',
  '0 10px 24px rgba(0, 0, 0, 0.11)',
  '0 12px 26px rgba(0, 0, 0, 0.12)',
  '0 12px 28px rgba(0, 0, 0, 0.12)',
  '0 14px 30px rgba(0, 0, 0, 0.13)',
  '0 14px 32px rgba(0, 0, 0, 0.13)',
  '0 16px 34px rgba(0, 0, 0, 0.13)',
  '0 16px 36px rgba(0, 0, 0, 0.13)',
  '0 18px 38px rgba(0, 0, 0, 0.14)',
  '0 18px 40px rgba(0, 0, 0, 0.14)',
  '0 20px 42px rgba(0, 0, 0, 0.15)',
  '0 20px 44px rgba(0, 0, 0, 0.15)',
  '0 22px 46px rgba(0, 0, 0, 0.15)',
]

const paperNodePalette: NodePalette = {
  workforce: '#555555',
  manager: '#333333',
  room: '#444444',
  blank: '#888888',
  agent: '#333333',
  context: '#555555',
  input: '#444444',
  step: '#888888',
  sub_workflow: '#555555',
}

// ═══════════════════════════════════════════════════════════════════════════
// Theme registry
// ═══════════════════════════════════════════════════════════════════════════

const THEMES: Record<ThemeId, ThemeDefinition> = {
  linen: {
    id: 'linen',
    label: 'Linen',
    muiMode: 'light',
    palette: linenPalette,
    custom: linenCustom,
    shadows: linenShadows,
    nodePalette: linenNodePalette,
  },
  paper: {
    id: 'paper',
    label: 'Paper',
    muiMode: 'light',
    palette: paperPalette,
    custom: paperCustom,
    shadows: paperShadows,
    nodePalette: paperNodePalette,
  },
  obsidian: {
    id: 'obsidian',
    label: 'Obsidian',
    muiMode: 'dark',
    palette: obsidianPalette,
    custom: obsidianCustom,
    shadows: obsidianShadows,
    nodePalette: obsidianNodePalette,
  },
  midnight: {
    id: 'midnight',
    label: 'Midnight',
    muiMode: 'dark',
    palette: midnightPalette,
    custom: midnightCustom,
    shadows: midnightShadows,
    nodePalette: midnightNodePalette,
  },
  slate: {
    id: 'slate',
    label: 'Slate',
    muiMode: 'dark',
    palette: slatePalette,
    custom: slateCustom,
    shadows: slateShadows,
    nodePalette: slateNodePalette,
  },
}

const THEME_LIST: readonly ThemeDefinition[] = THEME_IDS.map((id) => THEMES[id])

// ---------------------------------------------------------------------------
// Validation helper (for localStorage migration)
// ---------------------------------------------------------------------------

const isValidThemeId = (value: string): value is ThemeId =>
  value === 'linen' || value === 'paper' || value === 'obsidian' || value === 'midnight' || value === 'slate'

export { THEMES, THEME_LIST, THEME_IDS, DEFAULT_THEME_ID, isValidThemeId }
export type { ThemeId, ThemeDefinition, NodePalette }
