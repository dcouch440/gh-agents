import type { PaletteOptions } from '@mui/material/styles';

const lightPalette: PaletteOptions = {
  mode: 'light',
  primary: {
    main: '#ef6c00',
    light: '#ff9800',
    dark: '#e65100',
    contrastText: '#ffffff',
  },
  secondary: {
    main: '#26a69a',
    light: '#4db6ac',
    dark: '#00897b',
  },
  background: {
    default: '#ffffff',
    paper: '#f6f8fa',
  },
  text: {
    primary: '#1f2328',
    secondary: '#636c76',
    disabled: '#8b949e',
  },
  success: {
    main: '#1a7f37',
    light: '#2da44e',
    dark: '#116329',
  },
  warning: {
    main: '#bf8700',
    light: '#d4a72c',
    dark: '#9a6700',
  },
  error: {
    main: '#cf222e',
    light: '#e5534b',
    dark: '#a40e26',
  },
  info: {
    main: '#0969da',
    light: '#218bff',
    dark: '#0550ae',
  },
  divider: 'rgba(31, 35, 40, 0.15)',
};

const darkPalette: PaletteOptions = {
  mode: 'dark',
  primary: {
    main: '#f57c00',
    light: '#ffb74d',
    dark: '#e65100',
    contrastText: '#ffffff',
  },
  secondary: {
    main: '#4db6ac',
    light: '#80cbc4',
    dark: '#26a69a',
  },
  background: {
    default: '#0d1117',
    paper: '#161b22',
  },
  text: {
    primary: '#e6edf3',
    secondary: '#8b949e',
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
    main: '#58a6ff',
    light: '#79c0ff',
    dark: '#388bfd',
  },
  divider: 'rgba(240, 246, 252, 0.1)',
};

const getPalette = (mode: 'light' | 'dark'): PaletteOptions =>
  mode === 'light' ? lightPalette : darkPalette;

export { getPalette };
