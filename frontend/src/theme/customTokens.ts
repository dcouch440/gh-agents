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
  gridLineColor: string;
  canvasVignette: string;
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
  chromeBg: '#D47830',
  cavityBg: '#F5F3F0',
  bgHeader: '#FAF9F7',
  bgEditor: '#FCFBFA',
  bgPanel: '#F8F7F5',
  borderHover: 'rgba(45, 27, 14, 0.15)',
  textFaint: '#CFC7BB',
  hoverOverlay: 'rgba(45, 27, 14, 0.04)',
  separatorSubtle: 'rgba(45, 27, 14, 0.06)',
  activeTint: 'rgba(255, 150, 79, 0.08)',
  activeTintStrong: 'rgba(255, 150, 79, 0.14)',
  activeGradient: 'linear-gradient(90deg, #FF964F, #4E8A5A)',
  activeGradientVertical: 'linear-gradient(180deg, #FF964F, #4E8A5A)',
  activeGlow: 'drop-shadow(0 0 6px rgba(255, 150, 79, 0.35))',
  gridDotColor: 'rgba(45, 27, 14, 0.18)',
  gridLineColor: 'rgba(45, 27, 14, 0.07)',
  canvasVignette: 'rgba(45, 27, 14, 0.06)',
  minimapBg: 'rgba(240, 235, 227, 0.9)',
  minimapMask: 'rgba(249, 246, 241, 0.7)',
  floatingPanelBg: 'rgba(254, 252, 250, 0.96)',
  floatingPanelBorder: 'rgba(45, 27, 14, 0.12)',
  chromeText: 'rgba(254, 252, 250, 0.70)',
  chromeTextActive: '#FEFCFA',
  chromeTextHover: 'rgba(254, 252, 250, 0.90)',
  chromeActiveGlow: 'drop-shadow(0 0 4px rgba(255, 150, 79, 0.4))',
  chromeActiveBar: '#FEFCFA',
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
  gridDotColor: 'rgba(255, 255, 255, 0.05)',
  gridLineColor: 'rgba(255, 255, 255, 0.025)',
  canvasVignette: 'rgba(0, 0, 0, 0.15)',
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
