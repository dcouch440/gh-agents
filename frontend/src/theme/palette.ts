import type { PaletteOptions } from '@mui/material/styles';

const lightPalette: PaletteOptions = {
  mode: 'light',
  primary: {
    main: '#2563eb',
    light: '#3b82f6',
    dark: '#1d4ed8',
    contrastText: '#ffffff',
  },
  secondary: {
    main: '#0d9488',
    light: '#14b8a6',
    dark: '#0f766e',
  },
  background: {
    default: '#f8f9fb',
    paper: '#ffffff',
  },
  text: {
    primary: '#0f172a',
    secondary: '#64748b',
    disabled: '#94a3b8',
  },
  success: {
    main: '#16a34a',
    light: '#22c55e',
    dark: '#15803d',
  },
  warning: {
    main: '#d97706',
    light: '#f59e0b',
    dark: '#b45309',
  },
  error: {
    main: '#dc2626',
    light: '#ef4444',
    dark: '#b91c1c',
  },
  info: {
    main: '#2563eb',
    light: '#3b82f6',
    dark: '#1d4ed8',
  },
  divider: 'rgba(15, 23, 42, 0.08)',
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
