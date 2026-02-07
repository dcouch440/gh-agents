import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { AgentDetailPage } from './AgentDetailPage'
import type { Agent } from '@/types/agent'

const { mockUseAgent, mockUseAgentDocuments, mockUseDocuments, mockUseToolRouter, mockUseToolRouterMutations, mockUseRouterModes, mockUseRouterModeMutations, mockUseTools } = vi.hoisted(() => ({
  mockUseAgent: vi.fn(),
  mockUseAgentDocuments: vi.fn(),
  mockUseDocuments: vi.fn(),
  mockUseToolRouter: vi.fn(),
  mockUseToolRouterMutations: vi.fn(),
  mockUseRouterModes: vi.fn(),
  mockUseRouterModeMutations: vi.fn(),
  mockUseTools: vi.fn(),
}))

vi.mock('@/hooks/useAgents', () => ({
  useAgent: mockUseAgent,
}))

vi.mock('@/hooks/useAgentDocuments', () => ({
  useAgentDocuments: mockUseAgentDocuments,
}))

vi.mock('@/hooks/useDocuments', () => ({
  useDocuments: mockUseDocuments,
}))

vi.mock('@/hooks/useToolRouter', () => ({
  useToolRouter: mockUseToolRouter,
}))

vi.mock('@/hooks/useToolRouterMutations', () => ({
  useToolRouterMutations: mockUseToolRouterMutations,
}))

vi.mock('@/hooks/useRouterModes', () => ({
  useRouterModes: mockUseRouterModes,
}))

vi.mock('@/hooks/useRouterModeMutations', () => ({
  useRouterModeMutations: mockUseRouterModeMutations,
}))

vi.mock('@/hooks/useTools', () => ({
  useTools: mockUseTools,
}))

vi.mock('@/api', () => ({
  api: {
    agents: { update: vi.fn() },
  },
}))

const makeAgent = (id: string): Agent => ({
  id,
  name: `Agent ${id}`,
  system_prompt: 'Test prompt',
  model_provider: 'anthropic',
  model_id: 'claude-sonnet-4-20250514',
  model_max_tokens: 8192,
  model_temperature: 0.7,
  status: 'idle',
  output_schema_id: null,
  router_id: null,
  version: 1,
})

const defaultHookReturns = (agentId: string) => {
  mockUseAgent.mockReturnValue({
    agent: makeAgent(agentId),
    loading: false,
    error: null,
    reload: vi.fn(),
  })
  mockUseAgentDocuments.mockReturnValue({
    documents: [],
    loading: false,
    error: null,
    saving: false,
    addDocument: vi.fn(),
    removeDocument: vi.fn(),
  })
  mockUseDocuments.mockReturnValue({
    documents: [],
    loading: false,
    error: null,
  })
  mockUseToolRouter.mockReturnValue({
    router: null,
    loading: false,
    error: null,
    reload: vi.fn(),
  })
  mockUseToolRouterMutations.mockReturnValue({
    createRouter: vi.fn(),
    creating: false,
    updateRouter: vi.fn(),
    updating: false,
    deleteRouter: vi.fn(),
    deleting: false,
    loadRouterTools: vi.fn(),
    loadingTools: false,
    saveRouterTools: vi.fn(),
    savingTools: false,
    toolsError: null,
  })
  mockUseRouterModes.mockReturnValue({
    modes: [],
    loading: false,
    error: null,
    reload: vi.fn(),
  })
  mockUseRouterModeMutations.mockReturnValue({
    createMode: vi.fn(),
    creating: false,
    updateMode: vi.fn(),
    updating: false,
    deleteMode: vi.fn(),
    deleting: false,
    loadModeTools: vi.fn(),
    saveModeTools: vi.fn(),
    loadingTools: false,
    savingTools: false,
    toolsError: null,
  })
  mockUseTools.mockReturnValue({
    tools: [],
    loading: false,
    error: null,
  })
}

describe('AgentDetailPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders agent detail with id from params', async () => {
    defaultHookReturns('test-agent-id')

    render(
      <MemoryRouter initialEntries={['/agents/test-agent-id']}>
        <Routes>
          <Route path="/agents/:id" element={<AgentDetailPage />} />
        </Routes>
      </MemoryRouter>,
    )

    await waitFor(() => {
      expect(screen.getByText('Agent test-agent-id')).toBeInTheDocument()
    })
  })

  it('displays agent id from route params', async () => {
    defaultHookReturns('agent-123')

    render(
      <MemoryRouter initialEntries={['/agents/agent-123']}>
        <Routes>
          <Route path="/agents/:id" element={<AgentDetailPage />} />
        </Routes>
      </MemoryRouter>,
    )

    await waitFor(() => {
      expect(screen.getByText('Agent agent-123')).toBeInTheDocument()
    })
  })
})
