import '@testing-library/jest-dom/vitest'

// jsdom does not implement matchMedia, so provide a stub for components
// that use useReducedMotion or ThemeModeContext.
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})
