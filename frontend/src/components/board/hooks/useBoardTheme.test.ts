import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardTheme } from './useBoardTheme'

const { mockPaletteMode } = vi.hoisted(() => ({
  mockPaletteMode: { value: 'dark' as 'light' | 'dark' },
}))

vi.mock('@mui/material/styles', () => ({
  useTheme: () => ({ palette: { mode: mockPaletteMode.value } }),
}))

beforeEach(() => {
  mockPaletteMode.value = 'dark'
})

describe('useBoardTheme', () => {
  it('returns "dark" when MUI palette mode is dark', () => {
    mockPaletteMode.value = 'dark'
    const { result } = renderHook(() => useBoardTheme())
    expect(result.current).toBe('dark')
  })

  it('returns "light" when MUI palette mode is light', () => {
    mockPaletteMode.value = 'light'
    const { result } = renderHook(() => useBoardTheme())
    expect(result.current).toBe('light')
  })
})
