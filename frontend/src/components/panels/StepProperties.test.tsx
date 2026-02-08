import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { StepProperties } from './StepProperties'
import { mockWorkflowStep, mockAgent, mockPromptTemplate, mockOutputSchema } from '@/test/fixtures'

const {
  mockOpenRightPanel,
  mockAgentById,
  mockTemplateById,
  mockSchemaById,
} = vi.hoisted(() => ({
  mockOpenRightPanel: vi.fn(),
  mockAgentById: vi.fn(() => undefined),
  mockTemplateById: vi.fn(() => undefined),
  mockSchemaById: vi.fn(() => undefined),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  agentStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectById: () => mockAgentById,
  },
  promptTemplateStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectById: () => mockTemplateById,
  },
  outputSchemaStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectById: () => mockSchemaById,
  },
  layoutStore: {
    openRightPanel: mockOpenRightPanel,
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  mockAgentById.mockReturnValue(undefined)
  mockTemplateById.mockReturnValue(undefined)
  mockSchemaById.mockReturnValue(undefined)
})

describe('StepProperties', () => {
  describe('general section', () => {
    it('renders step name and execution mode', () => {
      render(<StepProperties step={mockWorkflowStep} />)
      expect(screen.getByText('First Step')).toBeInTheDocument()
      expect(screen.getByText('single')).toBeInTheDocument()
    })

    it('shows "Unnamed" when name is null', () => {
      const step = { ...mockWorkflowStep, name: null }
      render(<StepProperties step={step} />)
      expect(screen.getByText('Unnamed')).toBeInTheDocument()
    })
  })

  describe('agent section', () => {
    it('shows "None assigned" when no agent', () => {
      const step = { ...mockWorkflowStep, agent_id: 'missing' }
      render(<StepProperties step={step} />)
      const nones = screen.getAllByText('None assigned')
      expect(nones.length).toBeGreaterThanOrEqual(1)
    })

    it('renders agent name when assigned', () => {
      mockAgentById.mockReturnValue(mockAgent)
      render(<StepProperties step={mockWorkflowStep} />)
      expect(screen.getByText('TestBot')).toBeInTheDocument()
    })

    it('navigates to agents panel on action click', async () => {
      mockAgentById.mockReturnValue(mockAgent)
      const user = userEvent.setup()
      render(<StepProperties step={mockWorkflowStep} />)

      const buttons = screen.getAllByRole('button')
      const navButton = buttons.find((b) => b.querySelector('svg'))
      if (navButton) {
        await user.click(navButton)
        expect(mockOpenRightPanel).toHaveBeenCalledWith('agents')
      }
    })
  })

  describe('prompt template section', () => {
    it('shows "None assigned" when no template', () => {
      render(<StepProperties step={mockWorkflowStep} />)
      // Count "None assigned" occurrences — agent, template, and schema all unassigned
      const nones = screen.getAllByText('None assigned')
      expect(nones.length).toBeGreaterThanOrEqual(2)
    })

    it('renders template name when assigned', () => {
      mockTemplateById.mockReturnValue(mockPromptTemplate)
      const step = { ...mockWorkflowStep, prompt_template_id: 'template-001' }
      render(<StepProperties step={step} />)
      expect(screen.getByText('Test Template')).toBeInTheDocument()
      expect(screen.getByText('2 variable(s)')).toBeInTheDocument()
    })
  })

  describe('output schema section', () => {
    it('renders schema name when assigned', () => {
      mockSchemaById.mockReturnValue(mockOutputSchema)
      const step = { ...mockWorkflowStep, output_schema_id: 'schema-001' }
      render(<StepProperties step={step} />)
      expect(screen.getByText('Test Schema')).toBeInTheDocument()
    })
  })

  describe('position section', () => {
    it('renders position values', () => {
      const step = { ...mockWorkflowStep, position_x: 100, position_y: 200 }
      render(<StepProperties step={step} />)
      expect(screen.getByText('100')).toBeInTheDocument()
      expect(screen.getByText('200')).toBeInTheDocument()
    })
  })
})
