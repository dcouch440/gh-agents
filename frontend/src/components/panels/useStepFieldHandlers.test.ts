import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useStepFieldHandlers } from './useStepFieldHandlers'
import type { PromptTemplate } from '@/types'

const { mockPatchStepLocal } = vi.hoisted(() => ({
  mockPatchStepLocal: vi.fn(),
}))

vi.mock('@/stores', () => ({
  workflowStore: {
    patchStepLocal: mockPatchStepLocal,
  },
}))

// Clipboard mock
const mockWriteText = vi.fn(() => Promise.resolve())
Object.assign(navigator, { clipboard: { writeText: mockWriteText } })

const mockTemplate: PromptTemplate = {
  id: 'template-001',
  user_id: 'user-001',
  name: 'Test Template',
  description: 'A test prompt template',
  template: 'Hello {{name}}, please {{action}}',
  variables: ['name', 'action'],
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

const templatesMap = new Map<string, PromptTemplate>([['template-001', mockTemplate]])

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useStepFieldHandlers', () => {
  const setup = (stepId = 'step-001') =>
    renderHook(() => useStepFieldHandlers({ stepId, templatesMap }))

  describe('handleFieldChange', () => {
    it('patches step with name field', () => {
      const { result } = setup()
      act(() => {
        result.current.handleFieldChange('name', 'New Name')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { name: 'New Name' })
    })

    it('sets name to null when empty string', () => {
      const { result } = setup()
      act(() => {
        result.current.handleFieldChange('name', '')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { name: null })
    })

    it('patches step with prompt_template value as-is (never null)', () => {
      const { result } = setup()
      act(() => {
        result.current.handleFieldChange('prompt_template', '')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { prompt_template: '' })
    })

    it('patches step with system_prompt_suffix', () => {
      const { result } = setup()
      act(() => {
        result.current.handleFieldChange('system_prompt_suffix', 'Be careful')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { system_prompt_suffix: 'Be careful' })
    })
  })

  describe('handleAgentChange', () => {
    it('patches step with new agent_id', () => {
      const { result } = setup()
      act(() => {
        result.current.handleAgentChange('agent-002')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { agent_id: 'agent-002' })
    })

    it('does not patch when agent_id is null', () => {
      const { result } = setup()
      act(() => {
        result.current.handleAgentChange(null)
      })
      expect(mockPatchStepLocal).not.toHaveBeenCalled()
    })
  })

  describe('handleTemplateChange', () => {
    it('patches step with template id and resolved template text', () => {
      const { result } = setup()
      act(() => {
        result.current.handleTemplateChange('template-001')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', {
        prompt_template_id: 'template-001',
        prompt_template: 'Hello {{name}}, please {{action}}',
      })
    })

    it('clears template when null is passed', () => {
      const { result } = setup()
      act(() => {
        result.current.handleTemplateChange(null)
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', {
        prompt_template_id: null,
        prompt_template: '',
      })
    })

    it('uses empty string when template id not in map', () => {
      const { result } = setup()
      act(() => {
        result.current.handleTemplateChange('nonexistent')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', {
        prompt_template_id: 'nonexistent',
        prompt_template: '',
      })
    })
  })

  describe('handleSchemaChange', () => {
    it('patches step with new schema_id', () => {
      const { result } = setup()
      act(() => {
        result.current.handleSchemaChange('schema-001')
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { output_schema_id: 'schema-001' })
    })

    it('clears schema when null is passed', () => {
      const { result } = setup()
      act(() => {
        result.current.handleSchemaChange(null)
      })
      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { output_schema_id: null })
    })
  })

  describe('handleCopyVariable', () => {
    it('copies label to clipboard', () => {
      const { result } = setup()
      act(() => {
        result.current.handleCopyVariable('{step_output}')
      })
      expect(mockWriteText).toHaveBeenCalledWith('{step_output}')
    })
  })

  describe('memoization', () => {
    it('returns stable handler references across re-renders', () => {
      const { result, rerender } = setup()
      const firstHandlers = { ...result.current }
      rerender()
      expect(result.current.handleFieldChange).toBe(firstHandlers.handleFieldChange)
      expect(result.current.handleAgentChange).toBe(firstHandlers.handleAgentChange)
      expect(result.current.handleSchemaChange).toBe(firstHandlers.handleSchemaChange)
      expect(result.current.handleCopyVariable).toBe(firstHandlers.handleCopyVariable)
    })
  })
})
