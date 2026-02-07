import type { Shadows } from '@mui/material/styles';

const lightShadows: Shadows = [
  'none',
  '0 1px 2px rgba(0, 0, 0, 0.03)',
  '0 1px 3px rgba(0, 0, 0, 0.04)',
  '0 2px 4px rgba(0, 0, 0, 0.04)',
  '0 2px 6px rgba(0, 0, 0, 0.05)',
  '0 3px 8px rgba(0, 0, 0, 0.05)',
  '0 4px 10px rgba(0, 0, 0, 0.06)',
  '0 4px 12px rgba(0, 0, 0, 0.06)',
  '0 6px 14px rgba(0, 0, 0, 0.07)',
  '0 6px 16px rgba(0, 0, 0, 0.07)',
  '0 8px 18px rgba(0, 0, 0, 0.07)',
  '0 8px 20px rgba(0, 0, 0, 0.08)',
  '0 10px 22px rgba(0, 0, 0, 0.08)',
  '0 10px 24px rgba(0, 0, 0, 0.08)',
  '0 12px 26px rgba(0, 0, 0, 0.09)',
  '0 12px 28px rgba(0, 0, 0, 0.09)',
  '0 14px 30px rgba(0, 0, 0, 0.10)',
  '0 14px 32px rgba(0, 0, 0, 0.10)',
  '0 16px 34px rgba(0, 0, 0, 0.10)',
  '0 16px 36px rgba(0, 0, 0, 0.10)',
  '0 18px 38px rgba(0, 0, 0, 0.11)',
  '0 18px 40px rgba(0, 0, 0, 0.11)',
  '0 20px 42px rgba(0, 0, 0, 0.12)',
  '0 20px 44px rgba(0, 0, 0, 0.12)',
  '0 22px 46px rgba(0, 0, 0, 0.12)',
];

const darkShadows: Shadows = [
  'none',
  '0 1px 2px rgba(0, 0, 0, 0.30)',
  '0 1px 3px rgba(0, 0, 0, 0.32)',
  '0 2px 4px rgba(0, 0, 0, 0.32)',
  '0 2px 6px rgba(0, 0, 0, 0.34)',
  '0 3px 8px rgba(0, 0, 0, 0.34)',
  '0 4px 10px rgba(0, 0, 0, 0.36)',
  '0 4px 12px rgba(0, 0, 0, 0.36)',
  '0 6px 14px rgba(0, 0, 0, 0.38)',
  '0 6px 16px rgba(0, 0, 0, 0.38)',
  '0 8px 18px rgba(0, 0, 0, 0.38)',
  '0 8px 20px rgba(0, 0, 0, 0.40)',
  '0 10px 22px rgba(0, 0, 0, 0.40)',
  '0 10px 24px rgba(0, 0, 0, 0.40)',
  '0 12px 26px rgba(0, 0, 0, 0.42)',
  '0 12px 28px rgba(0, 0, 0, 0.42)',
  '0 14px 30px rgba(0, 0, 0, 0.44)',
  '0 14px 32px rgba(0, 0, 0, 0.44)',
  '0 16px 34px rgba(0, 0, 0, 0.44)',
  '0 16px 36px rgba(0, 0, 0, 0.44)',
  '0 18px 38px rgba(0, 0, 0, 0.46)',
  '0 18px 40px rgba(0, 0, 0, 0.46)',
  '0 20px 42px rgba(0, 0, 0, 0.48)',
  '0 20px 44px rgba(0, 0, 0, 0.48)',
  '0 22px 46px rgba(0, 0, 0, 0.50)',
];

const getShadows = (mode: 'light' | 'dark'): Shadows =>
  mode === 'light' ? lightShadows : darkShadows;

export { getShadows };
