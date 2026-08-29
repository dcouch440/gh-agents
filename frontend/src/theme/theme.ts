import { createTheme, type ThemeOptions } from '@mui/material/styles'
import { typography } from './typography'
import { getComponents } from './components'
import { THEMES } from './themes'
import type { ThemeId } from './themes'

const createAppTheme = (themeId: ThemeId) => {
  const def = THEMES[themeId]
  const options: ThemeOptions = {
    palette: { ...def.palette, custom: def.custom, nodePalette: def.nodePalette, statusPalette: def.statusPalette },
    typography,
    shape: { borderRadius: 10 },
    shadows: def.shadows,
    components: getComponents(def.muiMode, def.custom),
  }
  return createTheme(options)
}

export { createAppTheme }
