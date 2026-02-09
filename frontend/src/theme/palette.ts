import type { PaletteOptions } from '@mui/material/styles';

const lightPalette: PaletteOptions = {
  mode: 'light',
  primary: {
    main: '#FF964F',
    light: '#FFB480',
    dark: '#D47830',
    contrastText: '#2D1B0E',
  },
  secondary: {
    main: '#8B6548',
    light: '#A88066',
    dark: '#725438',
  },
  background: {
    default: '#F9F6F1',
    paper: '#FEFCFA',
  },
  text: {
    primary: '#2D1B0E',
    secondary: '#6B5742',
    disabled: '#A39283',
  },
  success: {
    main: '#4E8A5A',
    light: '#6BA878',
    dark: '#3B7046',
  },
  warning: {
    main: '#B87312',
    light: '#D49030',
    dark: '#955C0A',
  },
  error: {
    main: '#BF3326',
    light: '#D94A3D',
    dark: '#952820',
  },
  info: {
    main: '#FF964F',
    light: '#FFB480',
    dark: '#D47830',
  },
  divider: 'rgba(45, 27, 14, 0.10)',
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
