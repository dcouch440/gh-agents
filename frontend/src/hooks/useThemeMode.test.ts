import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useThemeMode } from './useThemeMode'

const { mockTheme, mockToggle, mockSetMode } = vi.hoisted(() => ({
  mockTheme: { value: 'dark' as 'light' | 'dark' },
  mockToggle: vi.fn(),
  mockSetMode: vi.fn(),
}))

vi.mock('@/stores/lib', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
}))

vi.mock('@/stores/uiStore', () => ({
  uiStore: {
    store: 'ui',
    selectTheme: () => mockTheme.value,
    toggleTheme: mockToggle,
    setTheme: mockSetMode,
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useThemeMode', () => {
  it('throws when used outside ThemeModeProvider', () => {
    expect(() => {
      renderHook(() => useThemeMode())
    }).toThrow('useThemeMode must be used within ThemeModeProvider')
  })
})
