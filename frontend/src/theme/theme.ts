import { createTheme, type ThemeOptions } from '@mui/material/styles'
import { getPalette } from './palette'
import { typography } from './typography'
import { getShadows } from './shadows'
import { getComponents } from './components'
import { getCustomTokens } from './customTokens'

const createAppTheme = (mode: 'light' | 'dark') => {
  const options: ThemeOptions = {
    palette: { ...getPalette(mode), custom: getCustomTokens(mode) },
    typography,
    shape: { borderRadius: 10 },
    shadows: getShadows(mode),
    components: getComponents(mode),
  }
  return createTheme(options)
}

export { createAppTheme }
