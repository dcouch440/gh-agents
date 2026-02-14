import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useNavigation } from './useNavigation'

const { mockPathname } = vi.hoisted(() => ({
  mockPathname: { value: '/' },
}))

vi.mock('react-router-dom', () => ({
  useLocation: () => ({ pathname: mockPathname.value }),
}))

beforeEach(() => {
  vi.clearAllMocks()
  mockPathname.value = '/'
})

describe('useNavigation', () => {
  it('returns navItems and utilityItems', () => {
    const { result } = renderHook(() => useNavigation())
    expect(result.current.navItems.length).toBeGreaterThan(0)
    expect(result.current.utilityItems.length).toBeGreaterThan(0)
  })

  it('partitions items into nav and utility groups', () => {
    const { result } = renderHook(() => useNavigation())
    for (const item of result.current.navItems) {
      expect(item.group).toBe('nav')
    }
    for (const item of result.current.utilityItems) {
      expect(item.group).toBe('utility')
    }
  })

  it('marks Dashboard active only on exact "/" match', () => {
    mockPathname.value = '/'
    const { result } = renderHook(() => useNavigation())
    const dashboard = result.current.navItems.find((i) => i.label === 'Dashboard')
    expect(dashboard?.isActive).toBe(true)
  })

  it('does not mark Dashboard active on "/agents"', () => {
    mockPathname.value = '/agents'
    const { result } = renderHook(() => useNavigation())
    const dashboard = result.current.navItems.find((i) => i.label === 'Dashboard')
    expect(dashboard?.isActive).toBe(false)
  })

  it('marks Agents active on prefix match "/agents/123"', () => {
    mockPathname.value = '/agents/123'
    const { result } = renderHook(() => useNavigation())
    const agents = result.current.navItems.find((i) => i.label === 'Agents')
    expect(agents?.isActive).toBe(true)
  })

  it('marks Settings active on exact match', () => {
    mockPathname.value = '/settings'
    const { result } = renderHook(() => useNavigation())
    const settings = result.current.utilityItems.find((i) => i.label === 'Settings')
    expect(settings?.isActive).toBe(true)
  })

  it('marks no nav items active on unrecognized path', () => {
    mockPathname.value = '/unknown'
    const { result } = renderHook(() => useNavigation())
    const activeNav = result.current.navItems.filter((i) => i.isActive)
    const activeUtility = result.current.utilityItems.filter((i) => i.isActive)
    expect(activeNav).toHaveLength(0)
    expect(activeUtility).toHaveLength(0)
  })
})
