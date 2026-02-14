import '@testing-library/jest-dom/vitest'

// jsdom does not implement ResizeObserver, so provide a no-op stub for
// components that use useNodeScale or ResizeObserver directly.
if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof globalThis.ResizeObserver
}

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
