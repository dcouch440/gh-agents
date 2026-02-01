import { renderHook, waitFor } from '@testing-library/react'
import { usePipelines, usePipelineRuns, usePipelineRun } from './usePipelines'
import { mockPipeline, mockPipelineRun, mockStageExecution } from '../test/fixtures'

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('../api', () => ({ api: { get: mockGet } }))
vi.mock('../constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('../constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('usePipelines', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('usePipelines', () => {
    it('fetches and returns pipelines', async () => {
      mockGet.mockResolvedValue([mockPipeline])
      const { result } = renderHook(() => usePipelines())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.pipelines).toEqual([mockPipeline])
    })

    it('sets error on failure', async () => {
      mockGet.mockRejectedValue(new Error('Failed'))
      const { result } = renderHook(() => usePipelines())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Failed')
    })
  })

  describe('usePipelineRuns', () => {
    it('fetches runs without filter', async () => {
      mockGet.mockResolvedValue([mockPipelineRun])
      const { result } = renderHook(() => usePipelineRuns())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.runs).toEqual([mockPipelineRun])
      expect(mockGet).toHaveBeenCalledWith('/pipeline-runs')
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
      mockGet.mockResolvedValue({ ...mockPipelineRun, stage_executions: [mockStageExecution] })
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
      expect(mockGet).not.toHaveBeenCalled()
    })
  })
})
