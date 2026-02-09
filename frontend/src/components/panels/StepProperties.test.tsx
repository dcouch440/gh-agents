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
  mockPatchStepLocal,
  mockFetchAllAgents,
  mockFetchIfStaleTemplates,
  mockFetchIfStaleSchemas,
  mockAgentById,
  mockAgentSelectAll,
  mockTemplateSelectAll,
  mockSchemaSelectAll,
  mockSelectEdges,
} = vi.hoisted(() => ({
  mockPatchStepLocal: vi.fn(),
  mockFetchAllAgents: vi.fn(() => Promise.resolve()),
  mockFetchIfStaleTemplates: vi.fn(() => Promise.resolve()),
  mockFetchIfStaleSchemas: vi.fn(() => Promise.resolve()),
  mockAgentById: vi.fn(() => undefined),
  mockAgentSelectAll: vi.fn(() => []),
  mockTemplateSelectAll: vi.fn(() => []),
  mockSchemaSelectAll: vi.fn(() => []),
  mockSelectEdges: vi.fn(() => []),
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
    patchStepLocal: mockPatchStepLocal,
    selectEdges: mockSelectEdges,
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
  mockAgentById.mockReturnValue(undefined)
  mockAgentSelectAll.mockReturnValue([mockAgent, mockAgent2])
  mockTemplateSelectAll.mockReturnValue([mockPromptTemplate, mockPromptTemplate2])
  mockSchemaSelectAll.mockReturnValue([mockOutputSchema, mockOutputSchema2])
  mockSelectEdges.mockReturnValue([])
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

    it('calls patchStepLocal when name changes', async () => {
      const user = userEvent.setup()
      renderStep()

      const input = screen.getByDisplayValue('First Step')
      await user.type(input, 'X')

      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { name: 'First StepX' })
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

    it('calls patchStepLocal with new agent_id on selection', async () => {
      const user = userEvent.setup()
      mockAgentById.mockReturnValue(mockAgent)
      renderStep()

      const agentSelect = screen.getAllByRole('combobox')[0]
      await user.click(agentSelect)

      const codeBot = screen.getByText('CodeBot')
      await user.click(codeBot)

      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { agent_id: 'agent-002' })
    })
  })

  describe('template tab', () => {
    it('renders CodeEditor with step prompt_template text', () => {
      renderStep()
      const editor = screen.getByPlaceholderText('Enter prompt template...')
      expect(editor).toHaveValue('{task_input}')
    })

    it('calls patchStepLocal when prompt changes', async () => {
      const user = userEvent.setup()
      renderStep()

      const editor = screen.getByPlaceholderText('Enter prompt template...')
      await user.type(editor, 'X')

      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { prompt_template: '{task_input}X' })
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

  })

  describe('output tab', () => {
    it('renders schema placeholder when no schema assigned', async () => {
      const user = userEvent.setup()
      renderStep()

      // Switch to output tab
      const outputTab = screen.getByText('Output')
      await user.click(outputTab)

      expect(screen.getByText('Select schema...')).toBeInTheDocument()
    })

    it('renders downstream steps in Outgoing section', async () => {
      const user = userEvent.setup()
      const step2 = { ...mockWorkflowStep, id: 'step-002', name: 'Downstream' }
      mockSelectEdges.mockReturnValue([
        { id: 'e1', from_step_id: 'step-001', to_step_id: 'step-002' },
      ])
      renderStep({ steps: [mockWorkflowStep, step2] })

      const outputTab = screen.getByText('Output')
      await user.click(outputTab)

      expect(screen.getByText('Outgoing')).toBeInTheDocument()
      expect(screen.getByText('Downstream')).toBeInTheDocument()
    })
  })

  describe('input tab', () => {
    it('shows empty state when no upstream steps', async () => {
      const user = userEvent.setup()
      renderStep()

      const inputTab = screen.getByText('Input')
      await user.click(inputTab)

      expect(screen.getByText('No incoming connections')).toBeInTheDocument()
    })

    it('renders upstream steps', async () => {
      const user = userEvent.setup()
      const step2 = { ...mockWorkflowStep, id: 'step-002', name: 'Upstream' }
      mockSelectEdges.mockReturnValue([
        { id: 'e1', from_step_id: 'step-002', to_step_id: 'step-001' },
      ])
      renderStep({ steps: [mockWorkflowStep, step2] })

      const inputTab = screen.getByText('Input')
      await user.click(inputTab)

      expect(screen.getByText('Upstream')).toBeInTheDocument()
    })

    it('renders upstream step schema when output_schema_id is set', async () => {
      const user = userEvent.setup()
      const step2 = {
        ...mockWorkflowStep,
        id: 'step-002',
        name: 'Upstream',
        output_schema_id: 'schema-001',
      }
      mockSelectEdges.mockReturnValue([
        { id: 'e1', from_step_id: 'step-002', to_step_id: 'step-001' },
      ])
      renderStep({ steps: [mockWorkflowStep, step2] })

      const inputTab = screen.getByText('Input')
      await user.click(inputTab)

      expect(screen.getByText('Test Schema')).toBeInTheDocument()
      expect(screen.getByText(/"result"/)).toBeInTheDocument()
    })
  })

  describe('system tab', () => {
    it('renders system prompt extension editor', async () => {
      const user = userEvent.setup()
      renderStep()

      const systemTab = screen.getByText('System')
      await user.click(systemTab)

      const editor = screen.getByPlaceholderText('Enter system prompt extension...')
      expect(editor).toBeInTheDocument()
      expect(editor).toHaveValue('')
    })

    it('renders system_prompt_suffix value when set', async () => {
      const user = userEvent.setup()
      const step = { ...mockWorkflowStep, system_prompt_suffix: 'Be extra careful.' }
      renderStep({ step })

      const systemTab = screen.getByText('System')
      await user.click(systemTab)

      const editor = screen.getByPlaceholderText('Enter system prompt extension...')
      expect(editor).toHaveValue('Be extra careful.')
    })

    it('calls patchStepLocal with system_prompt_suffix on change', async () => {
      const user = userEvent.setup()
      renderStep()

      const systemTab = screen.getByText('System')
      await user.click(systemTab)

      const editor = screen.getByPlaceholderText('Enter system prompt extension...')
      await user.type(editor, 'X')

      expect(mockPatchStepLocal).toHaveBeenCalledWith('step-001', { system_prompt_suffix: 'X' })
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
      // Name input should not be present — only the CodeEditor textareas remain
      expect(screen.queryByDisplayValue('First Step')).not.toBeInTheDocument()
    })

    it('does not render dropdown selects', () => {
      renderStep({ readOnly: true })
      expect(screen.queryByRole('combobox')).not.toBeInTheDocument()
    })

  })
})
