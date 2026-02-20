import type { ThemeOptions } from '@mui/material/styles'

type TypographyConfig = NonNullable<ThemeOptions['typography']>

const typography: TypographyConfig = {
  fontFamily: '"Nunito", "Inter", "Roboto", "Helvetica", "Arial", sans-serif',
  fontSize: 13,
  fontWeightLight: 300,
  fontWeightRegular: 400,
  fontWeightMedium: 500,
  fontWeightBold: 700,
  h1: {
    fontSize: '2rem',
    fontWeight: 600,
    lineHeight: 1.2,
    letterSpacing: '-0.025em',
  },
  h2: {
    fontSize: '1.625rem',
    fontWeight: 600,
    lineHeight: 1.3,
    letterSpacing: '-0.02em',
  },
  h3: {
    fontSize: '1.375rem',
    fontWeight: 600,
    lineHeight: 1.35,
    letterSpacing: '-0.015em',
  },
  h4: {
    fontSize: '1.125rem',
    fontWeight: 600,
    lineHeight: 1.4,
    letterSpacing: '-0.01em',
  },
  h5: {
    fontSize: '1rem',
    fontWeight: 600,
    lineHeight: 1.5,
  },
  h6: {
    fontSize: '0.875rem',
    fontWeight: 600,
    lineHeight: 1.5,
  },
  body1: {
    fontSize: '0.875rem',
    lineHeight: 1.5,
  },
  body2: {
    fontSize: '0.8125rem',
    lineHeight: 1.43,
  },
  caption: {
    fontSize: '0.6875rem',
    lineHeight: 1.5,
    letterSpacing: '0.01em',
  },
  overline: {
    fontSize: '0.625rem',
    fontWeight: 600,
    lineHeight: 1.5,
    letterSpacing: '0.06em',
    textTransform: 'uppercase',
  },
  button: {
    textTransform: 'none',
    fontWeight: 500,
    fontSize: '0.8125rem',
  },
}

export { typography }
