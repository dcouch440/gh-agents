import type { CustomTokens } from './customTokens'
import type { NodePalette } from './themes'

declare module '@mui/material/styles' {
  interface Palette {
    custom: CustomTokens
    nodePalette: NodePalette
  }
  interface PaletteOptions {
    custom?: CustomTokens
    nodePalette?: NodePalette
  }
}
