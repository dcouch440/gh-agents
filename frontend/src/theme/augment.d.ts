import type { CustomTokens } from './customTokens'
import type { NodePalette, StatusPalette } from './themes'

declare module '@mui/material/styles' {
  interface Palette {
    custom: CustomTokens
    nodePalette: NodePalette
    statusPalette: StatusPalette
  }
  interface PaletteOptions {
    custom?: CustomTokens
    nodePalette?: NodePalette
    statusPalette?: StatusPalette
  }
}
