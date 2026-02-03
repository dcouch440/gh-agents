import { render, screen, waitFor } from '@testing-library/react'
import { TaskProvider } from './TaskContext'
import { useTaskContext } from '@/hooks/useTaskContext'
import { mockTask } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

let wsHandler: ((data: unknown) => void) | null = null

vi.mock('@/hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    status: 'connected' as const,
    subscribe: (_channel: string, handler: (data: unknown) => void) => {
      wsHandler = handler
      return () => { wsHandler = null }
    },
  }),
}))

const { mockList } = vi.hoisted(() => ({ mockList: vi.fn() }))

vi.mock('@/api', () => ({
  api: { tasks: { list: mockList } },
}))

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { tasks, loading, error } = useTaskContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {tasks.map((t) => (
        <div key={t.id} data-testid={`task-${t.id}`}>
          {t.title}:{t.status}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('TaskContext', () => {
  describe('TaskProvider', () => {
    beforeEach(() => {
      wsHandler = null
      vi.clearAllMocks()
      mockList.mockResolvedValue({ items: [mockTask] })
    })

    it('fetches tasks on mount', async () => {
      render(
        <TaskProvider>
          <TestConsumer />
        </TaskProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toHaveTextContent('Test task:pending')
      })
    })

    it('updates a task status via WS partial update', async () => {
      render(
        <TaskProvider>
          <TestConsumer />
        </TaskProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toBeInTheDocument()
      })

      // Backend sends partial update: { id, status, progress, assigned_agent, user_id }
      wsHandler?.({ id: 'task-001', status: 'completed', progress: 1.0, assigned_agent: 'agent-001' })

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toHaveTextContent('Test task:completed')
      })
    })

    it('ignores WS update for unknown task id', async () => {
      render(
        <TaskProvider>
          <TestConsumer />
        </TaskProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toBeInTheDocument()
      })

      wsHandler?.({ id: 'task-999', status: 'completed', progress: null, assigned_agent: null })

      // Original task unchanged, no new task added
      expect(screen.getByTestId('task-task-001')).toHaveTextContent('Test task:pending')
      expect(screen.queryByTestId('task-task-999')).not.toBeInTheDocument()
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useTaskContext must be used within TaskProvider')
      spy.mockRestore()
    })
  })
})
