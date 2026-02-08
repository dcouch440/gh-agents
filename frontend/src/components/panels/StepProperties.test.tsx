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
  mockSelectUpstream,
  mockSelectDownstream,
} = vi.hoisted(() => ({
  mockUpdateStep: vi.fn(() => Promise.resolve(null)),
  mockFetchAllAgents: vi.fn(() => Promise.resolve()),
  mockFetchIfStaleTemplates: vi.fn(() => Promise.resolve()),
  mockFetchIfStaleSchemas: vi.fn(() => Promise.resolve()),
  mockAgentById: vi.fn(() => undefined),
  mockAgentSelectAll: vi.fn(() => []),
  mockTemplateSelectAll: vi.fn(() => []),
  mockSchemaSelectAll: vi.fn(() => []),
  mockSelectUpstream: vi.fn((): (_s: unknown) => readonly string[] => () => []),
  mockSelectDownstream: vi.fn((): (_s: unknown) => readonly string[] => () => []),
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
    store: { getState: vi.fn(), subscribe: vi.fn() },
    updateStep: mockUpdateStep,
    selectUpstream: mockSelectUpstream,
    selectDownstream: mockSelectDownstream,
  },
}))

vi.mock('@/utils/variableContext', () => ({
  buildVariableCompletions: vi.fn(() => []),
}))

vi.mock('@/utils/variableAutocomplete', () => ({
  createVariableAutocomplete: vi.fn(() => []),
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
      <StepProperties step={mockWorkflowStep} steps={[]} {...props} />
    </ThemeProvider>,
  )

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers({ shouldAdvanceTime: true })
  mockAgentById.mockReturnValue(undefined)
  mockAgentSelectAll.mockReturnValue([mockAgent, mockAgent2])
  mockTemplateSelectAll.mockReturnValue([mockPromptTemplate, mockPromptTemplate2])
  mockSchemaSelectAll.mockReturnValue([mockOutputSchema, mockOutputSchema2])
  mockSelectUpstream.mockReturnValue(() => [])
  mockSelectDownstream.mockReturnValue(() => [])
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

  describe('header', () => {
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

  describe('output variable name', () => {
    it('renders editable input for output_variable_name', () => {
      renderStep()
      const input = screen.getByPlaceholderText('e.g. parse_output')
      expect(input).toBeInTheDocument()
      expect(input).toHaveValue('')
    })

    it('renders value when output_variable_name is set', () => {
      const step = { ...mockWorkflowStep, output_variable_name: 'my_output' }
      renderStep({ step })
      const input = screen.getByPlaceholderText('e.g. parse_output')
      expect(input).toHaveValue('my_output')
    })

    it('calls updateStep with output_variable_name after debounce', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      renderStep()

      const input = screen.getByPlaceholderText('e.g. parse_output')
      await user.type(input, 'result')

      expect(mockUpdateStep).not.toHaveBeenCalled()

      await vi.advanceTimersByTimeAsync(500)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { output_variable_name: 'result' })
    })

    it('shows output variable as property row in readOnly mode', () => {
      const step = { ...mockWorkflowStep, output_variable_name: 'my_output' }
      renderStep({ step, readOnly: true })
      expect(screen.getByText('my_output')).toBeInTheDocument()
    })
  })

  describe('system prompt', () => {
    it('renders CodeEditor for system_prompt_suffix', () => {
      renderStep()
      const editor = screen.getByPlaceholderText('Enter system prompt suffix...')
      expect(editor).toBeInTheDocument()
      expect(editor).toHaveValue('')
    })

    it('renders system_prompt_suffix value when set', () => {
      const step = { ...mockWorkflowStep, system_prompt_suffix: 'Be extra careful.' }
      renderStep({ step })
      const editor = screen.getByPlaceholderText('Enter system prompt suffix...')
      expect(editor).toHaveValue('Be extra careful.')
    })

    it('calls updateStep with system_prompt_suffix after debounce', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      renderStep()

      const editor = screen.getByPlaceholderText('Enter system prompt suffix...')
      await user.type(editor, 'Think step by step')

      expect(mockUpdateStep).not.toHaveBeenCalled()

      await vi.advanceTimersByTimeAsync(500)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { system_prompt_suffix: 'Think step by step' })
    })

    it('renders hint text about appending to agent prompt', () => {
      renderStep()
      expect(screen.getByText('appends to agent prompt')).toBeInTheDocument()
    })
  })

  describe('prompt template', () => {
    it('renders section heading', () => {
      renderStep()
      expect(screen.getByText('Prompt Template')).toBeInTheDocument()
    })

    it('renders CodeEditor with step prompt_template text', () => {
      renderStep()
      const editor = screen.getByPlaceholderText('Enter prompt template...')
      expect(editor).toHaveValue('{task_input}')
    })

    it('calls updateStep with prompt_template after debounce', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      renderStep()

      const editor = screen.getByPlaceholderText('Enter prompt template...')
      await user.clear(editor)
      await user.type(editor, 'Do the thing')

      expect(mockUpdateStep).not.toHaveBeenCalled()

      await vi.advanceTimersByTimeAsync(500)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { prompt_template: 'Do the thing' })
    })

    it('renders template selector dropdown', () => {
      renderStep()
      expect(screen.getByText('Select template...')).toBeInTheDocument()
    })

    it('renders current template when assigned', () => {
      const step = { ...mockWorkflowStep, prompt_template_id: 'template-001' }
      renderStep({ step })
      expect(screen.getByText('Test Template')).toBeInTheDocument()
    })

    it('calls updateStep with prompt_template_id on template selection', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
      renderStep()

      // Template selector is the 3rd combobox: Agent (0), Schema (1), Template (2)
      const templateSelect = screen.getAllByRole('combobox')[2]
      await user.click(templateSelect)

      const reviewTemplate = screen.getByText('Code Review Template')
      await user.click(reviewTemplate)

      expect(mockUpdateStep).toHaveBeenCalledWith('step-001', { prompt_template_id: 'template-002' })
    })

    it('shows validation warning when prompt has no variable references', () => {
      const step = { ...mockWorkflowStep, prompt_template: 'No variables here' }
      renderStep({ step })
      expect(screen.getByText(/No variable references found/)).toBeInTheDocument()
    })

    it('hides validation warning when prompt contains variable references', () => {
      renderStep()
      // mockWorkflowStep has prompt_template: '{task_input}' which contains a variable
      expect(screen.queryByText(/No variable references found/)).not.toBeInTheDocument()
    })

    it('hides validation warning when prompt is empty', () => {
      const step = { ...mockWorkflowStep, prompt_template: '' }
      renderStep({ step })
      expect(screen.queryByText(/No variable references found/)).not.toBeInTheDocument()
    })
  })

  describe('position section removed', () => {
    it('does not render X or Y position fields', () => {
      const step = { ...mockWorkflowStep, position_x: 100, position_y: 200 }
      renderStep({ step })
      expect(screen.queryByText('Position')).not.toBeInTheDocument()
    })
  })

  describe('incoming connections', () => {
    it('renders upstream steps in Incoming section via store adjacency', () => {
      const step2 = { ...mockWorkflowStep, id: 'step-002', name: 'Upstream' }
      mockSelectUpstream.mockReturnValue(() => ['step-002'])
      renderStep({ steps: [mockWorkflowStep, step2] })
      expect(screen.getByText('Incoming')).toBeInTheDocument()
      expect(screen.getByText('Upstream')).toBeInTheDocument()
    })

    it('hides Incoming section when no upstream steps', () => {
      renderStep()
      expect(screen.queryByText('Incoming')).not.toBeInTheDocument()
    })
  })

  describe('outgoing connections', () => {
    it('renders downstream steps in Outgoing section via store adjacency', () => {
      const step2 = { ...mockWorkflowStep, id: 'step-002', name: 'Downstream' }
      mockSelectDownstream.mockReturnValue(() => ['step-002'])
      renderStep({ steps: [mockWorkflowStep, step2] })
      expect(screen.getByText('Outgoing')).toBeInTheDocument()
      expect(screen.getByText('Downstream')).toBeInTheDocument()
    })

    it('hides Outgoing section when no downstream steps', () => {
      renderStep()
      expect(screen.queryByText('Outgoing')).not.toBeInTheDocument()
    })
  })

  describe('readOnly mode', () => {
    it('renders name as plain text without editable input', () => {
      renderStep({ readOnly: true })
      expect(screen.getByText('First Step')).toBeInTheDocument()
      // Name input should not be present — only the CodeEditor textareas remain
      expect(screen.queryByDisplayValue('First Step')).not.toBeInTheDocument()
    })

    it('does not render dropdown selects', () => {
      renderStep({ readOnly: true })
      expect(screen.queryByRole('combobox')).not.toBeInTheDocument()
    })

    it('hides validation warning in readOnly mode', () => {
      const step = { ...mockWorkflowStep, prompt_template: 'No variables here' }
      renderStep({ step, readOnly: true })
      expect(screen.queryByText(/No variable references found/)).not.toBeInTheDocument()
    })
  })
})
