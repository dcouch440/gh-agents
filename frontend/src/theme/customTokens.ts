type CustomTokens = {
  chromeBg: string;
  cavityBg: string;
  bgHeader: string;
  bgEditor: string;
  bgPanel: string;
  borderHover: string;
  textFaint: string;
  hoverOverlay: string;
  separatorSubtle: string;
  activeTint: string;
  activeTintStrong: string;
  activeGradient: string;
  activeGradientVertical: string;
  activeGlow: string;
  gridDotColor: string;
  minimapBg: string;
  minimapMask: string;
  floatingPanelBg: string;
  floatingPanelBorder: string;
  chromeText: string;
  chromeTextActive: string;
  chromeTextHover: string;
  chromeActiveGlow: string;
  chromeActiveBar: string;
};

const lightCustomTokens: CustomTokens = {
  chromeBg: '#d09070',
  cavityBg: '#ebe5db',
  bgHeader: '#e8e2d8',
  bgEditor: '#f5f0e8',
  bgPanel: '#f0ebe3',
  borderHover: 'rgba(61, 43, 31, 0.12)',
  textFaint: '#d1c9be',
  hoverOverlay: 'rgba(61, 43, 31, 0.03)',
  separatorSubtle: 'rgba(61, 43, 31, 0.04)',
  activeTint: 'rgba(192, 80, 46, 0.06)',
  activeTintStrong: 'rgba(192, 80, 46, 0.10)',
  activeGradient: 'linear-gradient(90deg, #c0502e, #6b8f71)',
  activeGradientVertical: 'linear-gradient(180deg, #c0502e, #6b8f71)',
  activeGlow: 'drop-shadow(0 0 4px rgba(192, 80, 46, 0.3))',
  gridDotColor: 'rgba(61, 43, 31, 0.40)',
  minimapBg: 'rgba(235, 229, 219, 0.9)',
  minimapMask: 'rgba(245, 240, 232, 0.7)',
  floatingPanelBg: 'rgba(250, 247, 242, 0.95)',
  floatingPanelBorder: 'rgba(61, 43, 31, 0.12)',
  chromeText: 'rgba(61, 43, 31, 0.55)',
  chromeTextActive: '#3d2b1f',
  chromeTextHover: 'rgba(61, 43, 31, 0.78)',
  chromeActiveGlow: 'drop-shadow(0 0 3px rgba(61, 43, 31, 0.25))',
  chromeActiveBar: '#3d2b1f',
};

const darkCustomTokens: CustomTokens = {
  chromeBg: '#12161f',
  cavityBg: '#060a10',
  bgHeader: '#0d1017',
  bgEditor: '#0a0e14',
  bgPanel: '#0e1219',
  borderHover: 'rgba(240, 246, 252, 0.12)',
  textFaint: '#2d333b',
  hoverOverlay: 'rgba(255, 255, 255, 0.02)',
  separatorSubtle: 'rgba(240, 246, 252, 0.03)',
  activeTint: 'rgba(59, 130, 246, 0.04)',
  activeTintStrong: 'rgba(59, 130, 246, 0.06)',
  activeGradient: 'linear-gradient(90deg, #3b82f6, #2dd4bf)',
  activeGradientVertical: 'linear-gradient(180deg, #3b82f6, #2dd4bf)',
  activeGlow: 'drop-shadow(0 0 4px rgba(59, 130, 246, 0.4))',
  gridDotColor: 'rgba(255, 255, 255, 0.03)',
  minimapBg: 'rgba(6, 10, 16, 0.9)',
  minimapMask: 'rgba(0, 0, 0, 0.7)',
  floatingPanelBg: 'rgba(12, 16, 24, 0.92)',
  floatingPanelBorder: 'rgba(240, 246, 252, 0.1)',
  chromeText: '#7d8590',
  chromeTextActive: '#3b82f6',
  chromeTextHover: '#f0f6fc',
  chromeActiveGlow: 'drop-shadow(0 0 4px rgba(59, 130, 246, 0.4))',
  chromeActiveBar: '#3b82f6',
};

const getCustomTokens = (mode: 'light' | 'dark'): CustomTokens =>
  mode === 'light' ? lightCustomTokens : darkCustomTokens;

export { getCustomTokens };
export type { CustomTokens };
