export const FORM_NODE = {
  DEFAULT_WIDTH: 560,
  DEFAULT_HEIGHT: 500,
  MIN_WIDTH: 360,
  MIN_HEIGHT: 300,
  MAX_WIDTH: 1800,
  MAX_HEIGHT: 1600,
  HEADER_HEIGHT: 60,
  TAB_STRIP_HEIGHT: 32,
} as const

// --- Notch-based text scaling ---

export type ScaleNotch = 'XS' | 'S' | 'M' | 'L' | 'XL' | 'XXL'

export const NOTCH_ORDER: readonly ScaleNotch[] = ['XS', 'S', 'M', 'L', 'XL', 'XXL']

export const SCALE_NOTCH_ZOOM: Record<ScaleNotch, number> = {
  XS:  0.85,
  S:   0.95,
  M:   1.0,
  L:   1.12,
  XL:  1.25,
  XXL: 1.4,
} as const

export const WIDTH_BREAKPOINTS: ReadonlyArray<{ readonly maxWidth: number; readonly notch: ScaleNotch }> = [
  { maxWidth: 420,       notch: 'XS'  },
  { maxWidth: 559,       notch: 'S'   },
  { maxWidth: 780,       notch: 'M'   },
  { maxWidth: 1050,      notch: 'L'   },
  { maxWidth: 1400,      notch: 'XL'  },
  { maxWidth: Infinity,  notch: 'XXL' },
]

export const HEIGHT_BREAKPOINTS: ReadonlyArray<{ readonly maxHeight: number; readonly notch: ScaleNotch }> = [
  { maxHeight: 350,      notch: 'XS'  },
  { maxHeight: 499,      notch: 'S'   },
  { maxHeight: 700,      notch: 'M'   },
  { maxHeight: 1000,     notch: 'L'   },
  { maxHeight: 1300,     notch: 'XL'  },
  { maxHeight: Infinity, notch: 'XXL' },
]
