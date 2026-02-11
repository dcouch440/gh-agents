import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useStepVariableContext } from './useStepVariableContext'
import { mockWorkflowStep, mockOutputSchema } from '@/test/fixtures'
import type { WorkflowStep, OutputSchema } from '@/types'

const { mockBuildVariableCompletions, mockPatchStepSilent, mockCreateVariableAutocomplete } = vi.hoisted(() => ({
  mockBuildVariableCompletions: vi.fn(() => ({ completions: [], autoNamed: [] })),
  mockPatchStepSilent: vi.fn(),
  mockCreateVariableAutocomplete: vi.fn(() => []),
}))

vi.mock('@/stores', () => ({
  workflowStore: {
    patchStepSilent: mockPatchStepSilent,
  },
}))

vi.mock('@/utils/variableContext', () => ({
  buildVariableCompletions: mockBuildVariableCompletions,
}))

vi.mock('@/utils/variableAutocomplete', () => ({
  createVariableAutocomplete: mockCreateVariableAutocomplete,
}))

beforeEach(() => {
  vi.clearAllMocks()
})

const upstreamStep: WorkflowStep = {
  ...mockWorkflowStep,
  id: 'step-upstream',
  name: 'Data Loader',
  output_variable_name: null,
  output_schema_id: 'schema-001',
}

const stepsById = new Map<string, WorkflowStep>([
  [mockWorkflowStep.id, mockWorkflowStep],
  [upstreamStep.id, upstreamStep],
])

const schemasMap = new Map<string, OutputSchema>([['schema-001', mockOutputSchema]])

describe('useStepVariableContext', () => {
  const setup = (overrides: Partial<Parameters<typeof useStepVariableContext>[0]> = {}) =>
    renderHook(() =>
      useStepVariableContext({
        upstreamIds: [upstreamStep.id],
        stepsById,
        schemasMap,
        step: mockWorkflowStep,
        ...overrides,
      }),
    )

  it('calls buildVariableCompletions with correct arguments', () => {
    setup()
    expect(mockBuildVariableCompletions).toHaveBeenCalledWith(
      [upstreamStep.id],
      stepsById,
      schemasMap,
      mockWorkflowStep,
    )
  })

  it('returns variableContext from buildVariableCompletions', () => {
    const mockContext = {
      completions: [{ label: '{data_loader}', displayLabel: 'data_loader', detail: 'any', section: 'Data Loader' }],
      autoNamed: [],
    }
    mockBuildVariableCompletions.mockReturnValue(mockContext)

    const { result } = setup()
    expect(result.current.variableContext).toEqual(mockContext)
  })

  it('returns autocompleteExtension from createVariableAutocomplete', () => {
    const mockExtension = [{ id: 'mock-extension' }]
    mockCreateVariableAutocomplete.mockReturnValue(mockExtension)

    const { result } = setup()
    expect(result.current.autocompleteExtension).toBe(mockExtension)
  })

  it('creates autocompleteExtension only once (stable reference)', () => {
    const { result, rerender } = setup()
    const first = result.current.autocompleteExtension
    rerender()
    expect(result.current.autocompleteExtension).toBe(first)
    expect(mockCreateVariableAutocomplete).toHaveBeenCalledTimes(1)
  })

  describe('auto-naming', () => {
    it('patches upstream steps that need auto-derived variable names', () => {
      mockBuildVariableCompletions.mockReturnValue({
        completions: [],
        autoNamed: [{ stepId: 'step-upstream', derivedName: 'data_loader' }],
      })

      setup()

      expect(mockPatchStepSilent).toHaveBeenCalledWith('step-upstream', {
        output_variable_name: 'data_loader',
      })
    })

    it('does not patch when autoNamed is empty', () => {
      mockBuildVariableCompletions.mockReturnValue({
        completions: [],
        autoNamed: [],
      })

      setup()

      expect(mockPatchStepSilent).not.toHaveBeenCalled()
    })
  })

  describe('completions ref', () => {
    it('passes a getter function to createVariableAutocomplete', () => {
      setup()
      expect(mockCreateVariableAutocomplete).toHaveBeenCalledWith(expect.any(Function))
    })

    it('getter returns latest completions', () => {
      const completions = [
        { label: '{output}', displayLabel: 'output', detail: 'any', section: 'Test' },
      ]
      mockBuildVariableCompletions.mockReturnValue({ completions, autoNamed: [] })

      setup()

      // The getter is the first arg passed to createVariableAutocomplete
      const getter = mockCreateVariableAutocomplete.mock.calls[0][0] as () => unknown
      expect(getter()).toEqual(completions)
    })
  })
})
