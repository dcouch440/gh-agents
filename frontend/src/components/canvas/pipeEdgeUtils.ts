import { PIPE } from './constants'

type PipeOpacities = {
  glow: number
  body: number
  core: number
  particle: number
}

const parseHex = (hex: string): [number, number, number] => {
  const h = hex.startsWith('#') ? hex.slice(1) : hex
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ]
}

const toHex = (r: number, g: number, b: number): string => {
  const clamp = (v: number) => Math.round(Math.min(255, Math.max(0, v)))
  return `#${clamp(r).toString(16).padStart(2, '0')}${clamp(g).toString(16).padStart(2, '0')}${clamp(b).toString(16).padStart(2, '0')}`
}

/** Brighten a hex color by mixing toward white. factor 0 = unchanged, 1 = white. */
const brightenHex = (hex: string, factor: number): string => {
  const [r, g, b] = parseHex(hex)
  return toHex(
    r + (255 - r) * factor,
    g + (255 - g) * factor,
    b + (255 - b) * factor,
  )
}

/** Compute opacity values for each pipe layer based on protocol/selected state. */
const computePipeOpacities = (isProtocol: boolean, selected: boolean): PipeOpacities => {
  if (selected) {
    return {
      glow: PIPE.GLOW_OPACITY_SELECTED,
      body: PIPE.BODY_OPACITY_SELECTED,
      core: PIPE.CORE_OPACITY_SELECTED,
      particle: PIPE.PARTICLE_OPACITY_SELECTED,
    }
  }
  if (isProtocol) {
    return {
      glow: PIPE.GLOW_OPACITY,
      body: PIPE.BODY_OPACITY,
      core: PIPE.CORE_OPACITY,
      particle: PIPE.PARTICLE_OPACITY,
    }
  }
  return {
    glow: 0,
    body: PIPE.BODY_OPACITY_DIM,
    core: PIPE.CORE_OPACITY_DIM,
    particle: 0,
  }
}

export type { PipeOpacities }
export { brightenHex, computePipeOpacities }
