import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup } from '@/test/render'
import { Board } from './Board'
import { boardStore } from '@/stores'
import { INITIAL_STATE } from '@/stores/boardStore/_store'

// ── Excalidraw Mock ──────────────────────────────────────────────────────

const { capturedProps } = vi.hoisted(() => ({
  capturedProps: { current: null as Record<string, unknown> | null },
}))

vi.mock('@excalidraw/excalidraw', () => ({
  Excalidraw: (props: Record<string, unknown>) => {
    capturedProps.current = props
    return <div data-testid="excalidraw-mock" />
  },
}))

vi.mock('@excalidraw/excalidraw/index.css', () => ({}))

// ── Setup ────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  capturedProps.current = null
  boardStore.store.setState(INITIAL_STATE)
})

// ── Tests ────────────────────────────────────────────────────────────────

describe('Board', () => {
  it('renders Excalidraw and SubmitBar', () => {
    render(<Board workflowId="wf-1" />)

    expect(screen.getByTestId('excalidraw-mock')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /submit/i })).toBeInTheDocument()
  })

  it('passes theme to Excalidraw', () => {
    render(<Board workflowId="wf-1" />)

    // Test render uses 'midnight' theme (dark mode)
    expect(capturedProps.current).not.toBeNull()
    expect(capturedProps.current!['theme']).toBe('dark')
  })

  it('passes excalidrawAPI callback to Excalidraw', () => {
    render(<Board workflowId="wf-1" />)

    expect(capturedProps.current).not.toBeNull()
    expect(typeof capturedProps.current!['excalidrawAPI']).toBe('function')
  })

  it('calls boardStore.resetBoard on unmount', () => {
    const resetSpy = vi.spyOn(boardStore, 'resetBoard')
    render(<Board workflowId="wf-1" />)

    expect(resetSpy).not.toHaveBeenCalled()
    cleanup()
    expect(resetSpy).toHaveBeenCalledOnce()
  })
})
