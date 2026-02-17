import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { AgentDetailPage } from './AgentDetailPage'
import type { Agent } from '@/types/agent'

const { mockFetchOne, mockFetchContext, _agentState } = vi.hoisted(() => ({
  mockFetchOne: vi.fn(),
  mockFetchContext: vi.fn(),
  _agentState: { agent: undefined as Agent | undefined, context: [] as unknown[] },
}))

vi.mock('@/stores/agentStore', () => ({
  agentStore: {
    store: {
      getState: () => ({}),
      subscribe: () => () => {},
    },
    selectById: () => () => _agentState.agent,
    selectLoading: () => false,
    selectError: () => null,
    selectContext: () => () => _agentState.context,
    fetchAll: vi.fn().mockResolvedValue(undefined),
    fetchOne: mockFetchOne,
    fetchContext: mockFetchContext,
    setContext: vi.fn().mockResolvedValue(undefined),
    create: vi.fn(),
    update: vi.fn(),
    remove: vi.fn(),
  },
}))

vi.mock('@/stores/toolStore', () => {
  const emptyArray: never[] = []
  return {
    toolStore: {
      store: { getState: () => ({}), subscribe: () => () => {} },
      selectAll: () => emptyArray,
      selectLoading: () => false,
      fetchAll: vi.fn().mockResolvedValue(undefined),
    },
  }
})

vi.mock('@/stores/documentStore', () => {
  const emptyArray: never[] = []
  return {
    documentStore: {
      store: { getState: () => ({}), subscribe: () => () => {} },
      selectAll: () => emptyArray,
      selectLoading: () => false,
      fetchAll: vi.fn().mockResolvedValue(undefined),
    },
  }
})

vi.mock('@/api', () => ({
  api: {
    agents: { update: vi.fn(), getContext: vi.fn().mockResolvedValue({ documents: [] }) },
    tools: { list: vi.fn().mockResolvedValue({ items: [] }), get: vi.fn(), create: vi.fn(), update: vi.fn(), delete: vi.fn() },
    documents: {
      list: vi.fn().mockResolvedValue({ items: [] }),
      get: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
      search: vi.fn(),
    },
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
  version: 1,
})

describe('AgentDetailPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockFetchOne.mockResolvedValue(undefined)
    mockFetchContext.mockResolvedValue([])
  })

  it('renders agent detail with id from params', async () => {
    _agentState.agent = makeAgent('test-agent-id')

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
    _agentState.agent = makeAgent('agent-123')

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
