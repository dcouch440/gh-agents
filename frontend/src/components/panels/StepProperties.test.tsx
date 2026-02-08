import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeProvider, createTheme } from '@mui/material'
import { StepProperties } from './StepProperties'
import {
  mockWorkflowStep,
  mockAgent,
  mockAgent2,
  mockPromptTemplate,
  mockPromptTemplate2,
  mockOutputSchema,
  mockOutputSchema2,
} from '@/test/fixtures'

const theme = createTheme({ palette: { mode: 'dark' } })

const {
  mockUpdateStep,
  mockFetchAllAgents,
  mockFetchIfStaleTemplates,
  mockFetchIfStaleSchemas,
  mockAgentById,
  mockAgentSelectAll,
  mockTemplateSelectAll,
  mockSchemaSelectAll,
} = vi.hoisted(() => ({
  mockUpdateStep: vi.fn(() => Promise.resolve(null)),
  mockFetchAllAgents: vi.fn(() => Promise.resolve()),
  mockFetchIfStaleTemplates: vi.fn(() => Promise.resolve()),
  mockFetchIfStaleSchemas: vi.fn(() => Promise.resolve()),
  mockAgentById: vi.fn(() => undefined),
  mockAgentSelectAll: vi.fn(() => []),
  mockTemplateSelectAll: vi.fn(() => []),
  mockSchemaSelectAll: vi.fn(() => []),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  agentStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectById: () => mockAgentById,
    selectAll: mockAgentSelectAll,
    fetchAll: mockFetchAllAgents,
  },
  promptTemplateStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectAll: mockTemplateSelectAll,
    fetchIfStale: mockFetchIfStaleTemplates,
  },
  outputSchemaStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectAll: mockSchemaSelectAll,
    fetchIfStale: mockFetchIfStaleSchemas,
  },
  workflowStore: {
    updateStep: mockUpdateStep,
  },
}))

vi.mock('@/components/primitives', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/components/primitives')
  return {
    ...actual,
    CodeEditor: ({ value, onChange, placeholder, readOnly }: {
      value: string
      onChange: (v: string) => void
      placeholder?: string
      readOnly?: boolean
    }) => (
      <textarea
        data-testid="code-editor"
        value={value}
        onChange={(e) => { onChange(e.target.value) }}
        placeholder={placeholder}
        readOnly={readOnly}
      />
    ),
  }
})

const renderStep = (props: Partial<Parameters<typeof StepProperties>[0]> = {}) =>
  render(
    <ThemeProvider theme={theme}>
      <StepProperties step={mockWorkflowStep} {...props} />
    </ThemeProvider>,
  )

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers({ shouldAdvanceTime: true })
  mockAgentById.mockReturnValue(undefined)
  mockAgentSelectAll.mockReturnValue([mockAgent, mockAgent2])
  mockTemplateSelectAll.mockReturnValue([mockPromptTemplate, mockPromptTemplate2])
  mockSchemaSelectAll.mockReturnValue([mockOutputSchema, mockOutputSchema2])
})

afterEach(() => {
  vi.useRealTimers()
})

describe('StepProperties', () => {
  describe('data loading', () => {
    it('fetches agents, templates, and schemas on mount', () => {
      renderStep()
      expect(mockFetchAllAgents).toHaveBeenCalledOnce()
      expect(mockFetchIfStaleTemplates).toHaveBeenCalledOnce()
      expect(mockFetchIfStaleSchemas).toHaveBeenCalledOnce()
    })
  })

  describe('general section', () => {
    it('renders step name in an editable input', () => {
      renderStep()
      const input = screen.getByDisplayValue('First Step')
      expect(input).toBeInTheDocument()
    })

    it('shows placeholder when name is null', () => {
      const step = { ...mockWorkflowStep, name: null }
      renderStep({ step })
      const input = screen.getByPlaceholderText('Unnamed')
      expect(input).toBeInTheDocument()
    })

    it('calls updateStep after debounce when name changes', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      renderStep()

      const input = screen.getByDisplayValue('First Step')
      await user.clear(input)
      await user.type(input, 'New Name')

      expect(mockUpdateStep).not.toHaveBeenCalled()

      await vi.advanceTimersByTimeAsync(500)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { name: 'New Name' })
    })

    it('renders execution mode as a read-only badge', () => {
      renderStep()
      expect(screen.getByText('single')).toBeInTheDocument()
    })
  })

  describe('agent dropdown', () => {
    it('renders current agent as selected', () => {
      mockAgentById.mockReturnValue(mockAgent)
      renderStep()
      expect(screen.getByText('TestBot')).toBeInTheDocument()
    })

    it('calls updateStep with new agent_id on selection', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      mockAgentById.mockReturnValue(mockAgent)
      renderStep()

      const agentSelect = screen.getAllByRole('combobox')[0]
      await user.click(agentSelect)

      const codeBot = screen.getByText('CodeBot')
      await user.click(codeBot)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { agent_id: 'agent-002' })
    })
  })

  describe('prompt template dropdown', () => {
    it('renders placeholder when no template assigned', () => {
      renderStep()
      expect(screen.getByText('Select template...')).toBeInTheDocument()
    })

    it('renders current template when assigned', () => {
      const step = { ...mockWorkflowStep, prompt_template_id: 'template-001' }
      renderStep({ step })
      expect(screen.getByText('Test Template')).toBeInTheDocument()
    })

    it('calls updateStep with new prompt_template_id on selection', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      renderStep()

      const templateSelect = screen.getAllByRole('combobox')[1]
      await user.click(templateSelect)

      const reviewTemplate = screen.getByText('Code Review Template')
      await user.click(reviewTemplate)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { prompt_template_id: 'template-002' })
    })
  })

  describe('output schema dropdown', () => {
    it('renders placeholder when no schema assigned', () => {
      renderStep()
      expect(screen.getByText('Select schema...')).toBeInTheDocument()
    })

    it('renders current schema when assigned', () => {
      const step = { ...mockWorkflowStep, output_schema_id: 'schema-001' }
      renderStep({ step })
      expect(screen.getByText('Test Schema')).toBeInTheDocument()
    })
  })

  describe('system prompt', () => {
    it('renders CodeEditor with step prompt_template text', () => {
      renderStep()
      const editor = screen.getByTestId('code-editor')
      expect(editor).toHaveValue('{task_input}')
    })

    it('calls updateStep after debounce when prompt changes', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      renderStep()

      const editor = screen.getByTestId('code-editor')
      await user.clear(editor)
      await user.type(editor, 'Do the thing')

      expect(mockUpdateStep).not.toHaveBeenCalled()

      await vi.advanceTimersByTimeAsync(500)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { prompt_template: 'Do the thing' })
    })
  })

  describe('position section removed', () => {
    it('does not render X or Y position fields', () => {
      const step = { ...mockWorkflowStep, position_x: 100, position_y: 200 }
      renderStep({ step })
      expect(screen.queryByText('Position')).not.toBeInTheDocument()
    })
  })

  describe('readOnly mode', () => {
    it('renders name as plain text without editable input', () => {
      renderStep({ readOnly: true })
      expect(screen.getByText('First Step')).toBeInTheDocument()
      // Name input should not be present — only the CodeEditor textarea remains
      expect(screen.queryByDisplayValue('First Step')).not.toBeInTheDocument()
    })

    it('does not render dropdown selects', () => {
      renderStep({ readOnly: true })
      expect(screen.queryByRole('combobox')).not.toBeInTheDocument()
    })
  })
})
