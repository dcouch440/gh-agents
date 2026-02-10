import type { CustomTokens } from './customTokens'

declare module '@mui/material/styles' {
  interface Palette {
    custom: CustomTokens
  }
  interface PaletteOptions {
    custom?: CustomTokens
  }
}
