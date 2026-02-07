import { reviewQueueStore } from './reviewQueueStore'
import type { AgentExecution } from '@/types/execution'

const { mockList } = vi.hoisted(() => ({
  mockList: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    agentExecutions: {
      list: mockList,
    },
  },
}))

const exec1: AgentExecution = {
  id: 'e1',
  stage_execution_id: 'se1',
  agent_id: 'a1',
  workflow_step_id: null,
  is_interactive: true,
  parent_agent_execution_id: null,
  system_prompt_rendered: 'prompt',
  input: 'input',
  output: null,
  structured_output: null,
  selected_mode_id: null,
  status: 'awaiting_user',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: null,
}

const exec2: AgentExecution = {
  ...exec1,
  id: 'e2',
  stage_execution_id: 'se2',
}

beforeEach(() => {
  vi.clearAllMocks()
  reviewQueueStore.store.setState({
    executions: [],
    notification: null,
    loading: false,
    error: null,
  })
})

describe('reviewQueueStore', () => {
  describe('fetchPending', () => {
    it('populates executions from api', async () => {
      mockList.mockResolvedValue([exec1, exec2])
      await reviewQueueStore.fetchPending()

      const s = reviewQueueStore.store.getState()
      expect(s.executions).toEqual([exec1, exec2])
      expect(s.loading).toBe(false)
      expect(s.error).toBeNull()
    })

    it('sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Fetch failed'))
      await reviewQueueStore.fetchPending()

      const s = reviewQueueStore.store.getState()
      expect(s.error).toBe('Fetch failed')
      expect(s.loading).toBe(false)
    })
  })

  describe('addExecution', () => {
    it('prepends new execution and sets notification', () => {
      reviewQueueStore.addExecution(exec1)

      const s = reviewQueueStore.store.getState()
      expect(s.executions).toEqual([exec1])
      expect(s.notification).toEqual({
        id: 'e1',
        message: 'New review awaiting your approval',
      })
    })

    it('does not add duplicate', () => {
      reviewQueueStore.addExecution(exec1)
      reviewQueueStore.addExecution(exec1)

      expect(reviewQueueStore.store.getState().executions).toHaveLength(1)
    })
  })

  describe('removeExecution', () => {
    it('filters out execution by id', () => {
      reviewQueueStore.store.setState({ executions: [exec1, exec2] })
      reviewQueueStore.removeExecution('e1')

      expect(reviewQueueStore.store.getState().executions).toEqual([exec2])
    })
  })

  describe('dismissNotification', () => {
    it('clears notification', () => {
      reviewQueueStore.addExecution(exec1)
      expect(reviewQueueStore.store.getState().notification).not.toBeNull()

      reviewQueueStore.dismissNotification()
      expect(reviewQueueStore.store.getState().notification).toBeNull()
    })
  })

  describe('selectors', () => {
    it('selectPendingCount returns execution count', () => {
      reviewQueueStore.store.setState({ executions: [exec1, exec2] })
      expect(reviewQueueStore.selectPendingCount(reviewQueueStore.store.getState())).toBe(2)
    })

    it('selectExecutions returns executions array', () => {
      reviewQueueStore.store.setState({ executions: [exec1] })
      expect(reviewQueueStore.selectExecutions(reviewQueueStore.store.getState())).toEqual([exec1])
    })

    it('selectNotification returns null by default', () => {
      expect(reviewQueueStore.selectNotification(reviewQueueStore.store.getState())).toBeNull()
    })
  })
})
