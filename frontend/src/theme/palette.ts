import type { PaletteOptions } from '@mui/material/styles';

const lightPalette: PaletteOptions = {
  mode: 'light',
  primary: {
    main: '#c0502e',
    light: '#d4714e',
    dark: '#993e24',
    contrastText: '#ffffff',
  },
  secondary: {
    main: '#a0785a',
    light: '#b89070',
    dark: '#8b664c',
  },
  background: {
    default: '#f5f0e8',
    paper: '#faf7f2',
  },
  text: {
    primary: '#3d2b1f',
    secondary: '#7a6858',
    disabled: '#a89b8c',
  },
  success: {
    main: '#6b8f71',
    light: '#7fa886',
    dark: '#587a5e',
  },
  warning: {
    main: '#c27d2e',
    light: '#d4944a',
    dark: '#a06824',
  },
  error: {
    main: '#b5382a',
    light: '#d04a3c',
    dark: '#8f2c22',
  },
  info: {
    main: '#c0502e',
    light: '#d4714e',
    dark: '#993e24',
  },
  divider: 'rgba(61, 43, 31, 0.08)',
};

const darkPalette: PaletteOptions = {
  mode: 'dark',
  primary: {
    main: '#3b82f6',
    light: '#60a5fa',
    dark: '#2563eb',
    contrastText: '#ffffff',
  },
  secondary: {
    main: '#2dd4bf',
    light: '#5eead4',
    dark: '#14b8a6',
  },
  background: {
    default: '#080c12',
    paper: '#111318',
  },
  text: {
    primary: '#f0f6fc',
    secondary: '#7d8590',
    disabled: '#484f58',
  },
  success: {
    main: '#3fb950',
    light: '#56d364',
    dark: '#2ea043',
  },
  warning: {
    main: '#d29922',
    light: '#e3b341',
    dark: '#bb8009',
  },
  error: {
    main: '#f85149',
    light: '#ff7b72',
    dark: '#da3633',
  },
  info: {
    main: '#3b82f6',
    light: '#60a5fa',
    dark: '#2563eb',
  },
  divider: 'rgba(240, 246, 252, 0.06)',
};

const getPalette = (mode: 'light' | 'dark'): PaletteOptions =>
  mode === 'light' ? lightPalette : darkPalette;

export { getPalette };
