import { NOTCH_ORDER, SCALE_NOTCH_ZOOM, WIDTH_BREAKPOINTS, HEIGHT_BREAKPOINTS } from './constants'
import type { ScaleNotch } from './constants'

const resolveWidthNotch = (width: number): ScaleNotch => {
  for (const bp of WIDTH_BREAKPOINTS) {
    if (width <= bp.maxWidth) return bp.notch
  }
  return 'XXL'
}

const resolveHeightNotch = (height: number): ScaleNotch => {
  for (const bp of HEIGHT_BREAKPOINTS) {
    if (height <= bp.maxHeight) return bp.notch
  }
  return 'XXL'
}

const resolveScaleNotch = (width: number, height: number): ScaleNotch => {
  const wIdx = NOTCH_ORDER.indexOf(resolveWidthNotch(width))
  const hIdx = NOTCH_ORDER.indexOf(resolveHeightNotch(height))
  return NOTCH_ORDER[Math.min(wIdx, hIdx)]
}

const resolveScaleFactor = (width: number, height: number): number =>
  SCALE_NOTCH_ZOOM[resolveScaleNotch(width, height)]

export { resolveScaleNotch, resolveScaleFactor }
