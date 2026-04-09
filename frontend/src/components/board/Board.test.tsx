import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup } from '@/test/render'
import { Board } from './Board'
import { boardStore } from '@/stores'
import { INITIAL_STATE } from '@/stores/boardStore/_store'

// ── Mock useBoardElements to skip async fetch ─────────────────────────

vi.mock('./hooks/useBoardElements', () => ({
  useBoardElements: (_wfId: string, _setElements: unknown) => ({ loading: false }),
}))

// ── Mock useCanvasSync to avoid WebSocket dependency ──────────────────

vi.mock('./hooks/useCanvasSync', () => ({
  useCanvasSync: () => vi.fn(),
}))

// ── Setup ────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  boardStore.store.setState(INITIAL_STATE)
})

// ── Tests ────────────────────────────────────────────────────────────────

describe('Board', () => {
  it('renders Canvas and SubmitBar', () => {
    render(<Board workflowId="wf-1" />)

    expect(screen.getByRole('button', { name: /generate/i })).toBeInTheDocument()
  })

  it('renders toolbar with select, box, and arrow tools', () => {
    render(<Board workflowId="wf-1" />)

    expect(screen.getByRole('button', { name: /select \(v\)/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /box \(b\)/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /arrow \(a\)/i })).toBeInTheDocument()
  })

  it('renders zoom controls', () => {
    render(<Board workflowId="wf-1" />)

    expect(screen.getByRole('button', { name: /^zoom in$/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^zoom out$/i })).toBeInTheDocument()
  })

  it('calls boardStore.resetBoard on unmount', () => {
    const resetSpy = vi.spyOn(boardStore, 'resetBoard')
    render(<Board workflowId="wf-1" />)

    expect(resetSpy).not.toHaveBeenCalled()
    cleanup()
    expect(resetSpy).toHaveBeenCalledOnce()
  })
})
