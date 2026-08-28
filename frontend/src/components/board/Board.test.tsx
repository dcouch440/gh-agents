import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup, waitFor } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { Board } from './Board'
import { boardStore, workflowLiveStore } from '@/stores'
import { INITIAL_STATE } from '@/stores/boardStore/_store'
import type * as ApiModule from '@/api'

// `api` is frozen, so it has to be mocked at the module boundary.
const { mockGenerate } = vi.hoisted(() => ({ mockGenerate: vi.fn() }))

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof ApiModule>()
  return {
    ...actual,
    api: { ...actual.api, workflows: { ...actual.api.workflows, generate: mockGenerate } },
  }
})

// ── Mock useBoardElements to skip async fetch ─────────────────────────

vi.mock('./hooks/useBoardElements', () => ({
  useBoardElements: (_wfId: string, _setElements: unknown) => ({ loading: false }),
}))

// ── Mock useCanvasSync to avoid WebSocket dependency ──────────────────

const { mockFlushAndWait, mockHandleCanvasChange, flushOrder } = vi.hoisted(() => ({
  mockFlushAndWait: vi.fn(),
  mockHandleCanvasChange: vi.fn(),
  flushOrder: [] as string[],
}))

vi.mock('./hooks/useCanvasSync', () => ({
  useCanvasSync: () => ({
    handleCanvasChange: mockHandleCanvasChange,
    flushAndWait: mockFlushAndWait,
  }),
}))

// ── Setup ────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  flushOrder.length = 0
  boardStore.store.setState(INITIAL_STATE)
  mockGenerate.mockResolvedValue({ generating: 1 })
  workflowLiveStore.setGenerating(false)
  mockFlushAndWait.mockImplementation(() => {
    flushOrder.push('flush')
    return Promise.resolve()
  })
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

  // ── Generate ────────────────────────────────────────────────────────────
  //
  // The server builds its work list from persisted steps, so the board has to
  // be durable before generate runs. Skipping the wait is what made a
  // hand-drawn node need a second click.

  describe('generate', () => {
    it('waits for canvas sync to settle before POSTing generate', async () => {
      mockGenerate.mockImplementation(() => {
        flushOrder.push('generate')
        return Promise.resolve({ generating: 1 })
      })

      render(<Board workflowId="wf-1" />)
      await userEvent.click(screen.getByRole('button', { name: /generate/i }))

      await waitFor(() => { expect(mockGenerate).toHaveBeenCalledWith('wf-1') })
      expect(flushOrder).toEqual(['flush', 'generate'])
    })

    it('clears the spinner when the server queued nothing', async () => {
      mockGenerate.mockResolvedValue({ generating: 0 })

      render(<Board workflowId="wf-1" />)
      await userEvent.click(screen.getByRole('button', { name: /generate/i }))

      await waitFor(() => {
        expect(workflowLiveStore.selectIsGenerating(workflowLiveStore.store.getState())).toBe(false)
      })
    })

    it('keeps spinning when the server queued work', async () => {
      mockGenerate.mockResolvedValue({ generating: 3 })

      render(<Board workflowId="wf-1" />)
      await userEvent.click(screen.getByRole('button', { name: /generate/i }))

      await waitFor(() => { expect(mockGenerate).toHaveBeenCalled() })
      expect(workflowLiveStore.selectIsGenerating(workflowLiveStore.store.getState())).toBe(true)
    })
  })

  it('no longer offers undo/redo controls', () => {
    render(<Board workflowId="wf-1" />)

    expect(screen.queryByRole('button', { name: /undo/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /redo/i })).not.toBeInTheDocument()
  })
})
