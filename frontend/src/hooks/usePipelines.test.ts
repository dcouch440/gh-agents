import { renderHook, waitFor } from '@testing-library/react'
import { usePipelines, usePipelineRuns, usePipelineRun } from './usePipelines'
import { mockPipeline, mockPipelineRun, mockStageExecution } from '@/test/fixtures'

const { mockPipelinesList, mockRunsList, mockRunGet, mockGet } = vi.hoisted(() => ({
  mockPipelinesList: vi.fn(),
  mockRunsList: vi.fn(),
  mockRunGet: vi.fn(),
  mockGet: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    pipelines: { list: mockPipelinesList },
    pipelineRuns: { list: mockRunsList, get: mockRunGet },
    get: mockGet,
  },
}))
describe('usePipelines', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('usePipelines', () => {
    it('fetches and returns pipelines', async () => {
      mockPipelinesList.mockResolvedValue({ items: [mockPipeline] })
      const { result } = renderHook(() => usePipelines())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.pipelines).toEqual([mockPipeline])
    })

    it('sets error on failure', async () => {
      mockPipelinesList.mockRejectedValue(new Error('Failed'))
      const { result } = renderHook(() => usePipelines())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Failed')
    })
  })

  describe('usePipelineRuns', () => {
    it('fetches runs without filter', async () => {
      mockRunsList.mockResolvedValue({ items: [mockPipelineRun] })
      const { result } = renderHook(() => usePipelineRuns())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.runs).toEqual([mockPipelineRun])
      expect(mockRunsList).toHaveBeenCalled()
    })

    it('fetches runs with pipeline id filter', async () => {
      mockGet.mockResolvedValue([mockPipelineRun])
      renderHook(() => usePipelineRuns('pipeline-001'))

      await waitFor(() => {
        expect(mockGet).toHaveBeenCalledWith('/pipeline-runs?pipeline_id=pipeline-001')
      })
    })
  })

  describe('usePipelineRun', () => {
    it('fetches run with stage executions', async () => {
      mockRunGet.mockResolvedValue({ ...mockPipelineRun, stage_executions: [mockStageExecution] })
      const { result } = renderHook(() => usePipelineRun('run-001'))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.run).toEqual(mockPipelineRun)
      expect(result.current.executions).toEqual([mockStageExecution])
    })

    it('returns null when id is null', async () => {
      const { result } = renderHook(() => usePipelineRun(null))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.run).toBeNull()
      expect(result.current.executions).toEqual([])
      expect(mockRunGet).not.toHaveBeenCalled()
    })
  })
})
