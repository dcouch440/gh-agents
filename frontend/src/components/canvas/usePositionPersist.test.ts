import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { usePositionPersist } from './usePositionPersist'

const { mockUpdateStep } = vi.hoisted(() => ({
  mockUpdateStep: vi.fn(),
}))

vi.mock('@/stores', () => ({
  workflowStore: {
    updateStep: mockUpdateStep,
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

describe('usePositionPersist', () => {
  it('returns onNodeDragStop callback', () => {
    const { result } = renderHook(() => usePositionPersist())
    expect(typeof result.current.onNodeDragStop).toBe('function')
  })

  it('calls updateStep with rounded position on drag stop', () => {
    const { result } = renderHook(() => usePositionPersist())

    const mockEvent = {} as React.MouseEvent
    const mockNode = {
      id: 'step-001',
      position: { x: 100.7, y: 200.3 },
      data: {},
    }

    result.current.onNodeDragStop(mockEvent, mockNode as never)

    expect(mockUpdateStep).toHaveBeenCalledWith('step-001', {
      position_x: 101,
      position_y: 200,
    })
  })

  it('persists latest position for same node', () => {
    const { result } = renderHook(() => usePositionPersist())

    const mockEvent = {} as React.MouseEvent
    const node1 = { id: 'step-001', position: { x: 50, y: 50 }, data: {} }
    const node2 = { id: 'step-001', position: { x: 100, y: 200 }, data: {} }

    result.current.onNodeDragStop(mockEvent, node1 as never)
    result.current.onNodeDragStop(mockEvent, node2 as never)

    // Both calls go through since flush is immediate
    expect(mockUpdateStep).toHaveBeenCalledTimes(2)
    expect(mockUpdateStep).toHaveBeenLastCalledWith('step-001', {
      position_x: 100,
      position_y: 200,
    })
  })
})
