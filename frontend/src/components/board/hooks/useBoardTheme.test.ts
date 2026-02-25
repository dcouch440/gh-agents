import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardTheme } from './useBoardTheme'

const mockCustom = {
  canvasBg: '#0d1117',
  gridDotColor: 'rgba(240, 246, 252, 0.06)',
  connectorColor: '#30363d',
  surfaceBg: '#21262d',
  accent: '#3b82f6',
}

const { mockPaletteMode } = vi.hoisted(() => ({
  mockPaletteMode: { value: 'dark' as 'light' | 'dark' },
}))

vi.mock('@mui/material/styles', () => ({
  useTheme: () => ({
    palette: {
      mode: mockPaletteMode.value,
      text: { primary: '#ffffff' },
      custom: mockCustom,
    },
  }),
}))

beforeEach(() => {
  mockPaletteMode.value = 'dark'
})

describe('useBoardTheme', () => {
  it('returns canvas theme tokens', () => {
    const { result } = renderHook(() => useBoardTheme())
    expect(result.current.canvasBg).toBe('#0d1117')
    expect(result.current.connectorColor).toBe('#30363d')
    expect(result.current.accent).toBe('#3b82f6')
  })

  it('returns text color from palette', () => {
    const { result } = renderHook(() => useBoardTheme())
    expect(result.current.textPrimary).toBe('#ffffff')
  })
})
